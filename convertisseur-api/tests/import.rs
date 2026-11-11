mod common;

use axum::http::{Method, StatusCode};
use common::{PASSWORD, Server, create_account};
use convertisseur_api::{
    ensure_super_admin,
    import::{Catalogue, ImportReport, Tally},
    import_content,
};
use serde_json::json;
use sqlx::{AssertSqlSafe, PgPool};
use uuid::Uuid;

/// Status, created by, updated by, validated by.
type Review = (String, Option<Uuid>, Option<Uuid>, Option<Uuid>);

struct CardRow {
    id: Uuid,
    lesson_id: Uuid,
    position: i32,
    arabic: String,
    transliteration: Option<String>,
    meaning_fr: Option<String>,
    variety: String,
}

async fn card(pool: &PgPool, key: &str) -> CardRow {
    let (id, lesson_id, position, arabic, transliteration, meaning_fr, variety) = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            i32,
            String,
            Option<String>,
            Option<String>,
            String,
        ),
    >(
        "SELECT id, lesson_id, position, arabic, transliteration, meaning_fr, variety::text \
         FROM cards WHERE import_key = $1",
    )
    .bind(key)
    .fetch_one(pool)
    .await
    .unwrap();
    CardRow {
        id,
        lesson_id,
        position,
        arabic,
        transliteration,
        meaning_fr,
        variety,
    }
}

async fn status(pool: &PgPool, table: &str, key: &str) -> (String, Option<Uuid>) {
    sqlx::query_as(AssertSqlSafe(format!(
        "SELECT status::text, validated_by FROM {table} WHERE import_key = $1"
    )))
    .bind(key)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn count(pool: &PgPool, table: &str) -> usize {
    let count: i64 = sqlx::query_scalar(AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
        .fetch_one(pool)
        .await
        .unwrap();
    usize::try_from(count).unwrap()
}

fn expected_tallies(created: bool) -> ImportReport {
    let (units, lessons, cards) = Catalogue::embedded().unwrap().counts();
    let tally = |n| {
        if created {
            Tally {
                created: n,
                skipped: 0,
            }
        } else {
            Tally {
                created: 0,
                skipped: n,
            }
        }
    };
    ImportReport {
        units: tally(units),
        lessons: tally(lessons),
        cards: tally(cards),
    }
}

async fn super_admin(pool: &PgPool) -> Uuid {
    ensure_super_admin(pool, "admin", PASSWORD.to_owned())
        .await
        .unwrap()
}

#[sqlx::test]
async fn the_first_import_creates_everything_to_review(pool: PgPool) {
    let author = super_admin(&pool).await;

    let report = import_content(&pool).await.unwrap();

    assert_eq!(report, expected_tallies(true));
    for table in ["units", "lessons", "cards"] {
        let rows: Vec<Review> = sqlx::query_as(AssertSqlSafe(format!(
            "SELECT status::text, created_by, updated_by, validated_by FROM {table}"
        )))
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_ne!(rows.len(), 0, "{table}");
        for row in rows {
            assert_eq!(
                row,
                ("to_review".to_owned(), Some(author), Some(author), None)
            );
        }
    }
    assert_eq!(count(&pool, "units").await, report.units.created);
    assert_eq!(count(&pool, "lessons").await, report.lessons.created);
    assert_eq!(count(&pool, "cards").await, report.cards.created);

    let verse = card(&pool, "islam/coran/al-fatiha/aya-3").await;
    assert_eq!(verse.arabic, "مَلِكِ يَوْمِ الدِّينِ");
    assert_eq!(verse.position, 4);
    assert_eq!(verse.variety, "classique");
    let greeting = card(&pool, "arabe/darija/salutations/wach-rak").await;
    assert_eq!(greeting.variety, "darija");
    assert_eq!(greeting.transliteration.as_deref(), Some("wach rak ?"));
    assert_eq!(
        greeting.meaning_fr.as_deref(),
        Some("Comment vas-tu ? (à un homme)")
    );
}

#[sqlx::test]
async fn a_second_import_creates_nothing(pool: PgPool) {
    super_admin(&pool).await;
    import_content(&pool).await.unwrap();

    let report = import_content(&pool).await.unwrap();

    assert_eq!(report, expected_tallies(false));
    let (units, lessons, cards) = Catalogue::embedded().unwrap().counts();
    assert_eq!(count(&pool, "units").await, units);
    assert_eq!(count(&pool, "lessons").await, lessons);
    assert_eq!(count(&pool, "cards").await, cards);
}

#[sqlx::test]
async fn edits_and_validations_made_in_the_admin_area_are_kept(pool: PgPool) {
    super_admin(&pool).await;
    let reviewer = create_account(&pool, "relectrice", "admin").await;
    import_content(&pool).await.unwrap();
    let server = Server::new(&pool);
    let cookie = server.session("relectrice").await;
    let edited = card(&pool, "arabe/darija/salutations/labes").await;
    let answer = server
        .send(
            Method::PUT,
            &format!("/api/v1/cards/{}", edited.id),
            Some(&cookie),
            Some(&json!({
                "lessonId": edited.lesson_id,
                "arabic": "لاباس عليك",
                "transliteration": "labès 3lik",
                "meaningFr": "Ça va bien",
                "variety": "darija",
                "position": 1,
            })),
        )
        .await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    let validated = card(&pool, "islam/formules/formules/bismillah").await;
    let answer = server
        .send(
            Method::PUT,
            &format!("/api/v1/cards/{}/status", validated.id),
            Some(&cookie),
            Some(&json!({ "status": "validated" })),
        )
        .await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

    let report = import_content(&pool).await.unwrap();

    assert_eq!(report, expected_tallies(false));
    let after = card(&pool, "arabe/darija/salutations/labes").await;
    assert_eq!(after.id, edited.id);
    assert_eq!(after.position, 1);
    assert_eq!(after.arabic, "لاباس عليك");
    assert_eq!(after.transliteration.as_deref(), Some("labès 3lik"));
    assert_eq!(after.meaning_fr.as_deref(), Some("Ça va bien"));
    assert_eq!(
        status(&pool, "cards", "islam/formules/formules/bismillah").await,
        ("validated".to_owned(), Some(reviewer))
    );
}

#[sqlx::test]
async fn a_card_deleted_in_the_admin_area_comes_back_last_to_review(pool: PgPool) {
    super_admin(&pool).await;
    create_account(&pool, "relectrice", "admin").await;
    import_content(&pool).await.unwrap();
    let server = Server::new(&pool);
    let cookie = server.session("relectrice").await;
    let deleted = card(&pool, "arabe/alphabet/lettres-1/ba").await;
    let answer = server
        .send(
            Method::DELETE,
            &format!("/api/v1/cards/{}", deleted.id),
            Some(&cookie),
            None,
        )
        .await;
    assert_eq!(answer.status, StatusCode::NO_CONTENT, "{}", answer.body);

    let report = import_content(&pool).await.unwrap();

    let mut expected = expected_tallies(false);
    expected.cards.created = 1;
    expected.cards.skipped -= 1;
    assert_eq!(report, expected);
    let recreated = card(&pool, "arabe/alphabet/lettres-1/ba").await;
    assert_ne!(recreated.id, deleted.id);
    assert_eq!(recreated.lesson_id, deleted.lesson_id);
    assert_eq!(recreated.arabic, "ب");
    assert_eq!(recreated.position, 7);
    assert_eq!(
        status(&pool, "cards", "arabe/alphabet/lettres-1/ba").await,
        ("to_review".to_owned(), None)
    );
    let alif = card(&pool, "arabe/alphabet/lettres-1/alif").await;
    let ta = card(&pool, "arabe/alphabet/lettres-1/ta").await;
    assert_eq!((alif.position, ta.position), (1, 2));
}

#[sqlx::test]
async fn a_lesson_moved_in_the_admin_area_keeps_its_new_cards(pool: PgPool) {
    super_admin(&pool).await;
    import_content(&pool).await.unwrap();
    let lesson: Uuid =
        sqlx::query_scalar("SELECT id FROM lessons WHERE import_key = 'islam/coran/an-nas'")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("DELETE FROM cards WHERE import_key = 'islam/coran/an-nas/aya-6'")
        .execute(&pool)
        .await
        .unwrap();
    let other_unit: Uuid =
        sqlx::query_scalar("SELECT id FROM units WHERE import_key = 'islam/formules'")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("UPDATE lessons SET unit_id = $1, position = 2 WHERE id = $2")
        .bind(other_unit)
        .bind(lesson)
        .execute(&pool)
        .await
        .unwrap();

    import_content(&pool).await.unwrap();

    let unit_of_lesson: Uuid = sqlx::query_scalar("SELECT unit_id FROM lessons WHERE id = $1")
        .bind(lesson)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(unit_of_lesson, other_unit);
    assert_eq!(
        card(&pool, "islam/coran/an-nas/aya-6").await.lesson_id,
        lesson
    );
}

#[sqlx::test]
async fn refuses_to_import_without_a_super_admin(pool: PgPool) {
    let error = import_content(&pool).await.unwrap_err();

    assert!(error.to_string().contains("no super admin"), "{error:#}");
    assert_eq!(count(&pool, "units").await, 0);
}
