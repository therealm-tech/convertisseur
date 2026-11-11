mod common;

use axum::http::{Method, StatusCode};
use common::{
    PASSWORD, SALUTATIONS, Server, create_account, imported_id, sync_files, sync_salutations,
};
use convertisseur_api::{
    ensure_super_admin,
    storage::{MemoryStore, ObjectStore, Storage},
    sync::{Catalogue, HeldByRemovedEntries, SyncReport, Tally},
    sync_content,
};
use serde_json::json;
use sqlx::{AssertSqlSafe, PgPool};
use uuid::Uuid;

/// Status, created by, updated by, validated by.
type Review = (String, Option<Uuid>, Option<Uuid>, Option<Uuid>);

struct CardRow {
    id: Uuid,
    position: i32,
    arabic: String,
    transliteration: Option<String>,
    meaning_fr: Option<String>,
    variety: String,
}

async fn card(pool: &PgPool, key: &str) -> CardRow {
    let (id, position, arabic, transliteration, meaning_fr, variety) =
        sqlx::query_as::<_, (Uuid, i32, String, Option<String>, Option<String>, String)>(
            "SELECT id, position, arabic, transliteration, meaning_fr, variety::text \
         FROM cards WHERE import_key = $1",
        )
        .bind(key)
        .fetch_one(pool)
        .await
        .unwrap();
    CardRow {
        id,
        position,
        arabic,
        transliteration,
        meaning_fr,
        variety,
    }
}

/// Status, validator and review note of the row imported under `key`.
async fn review(pool: &PgPool, table: &str, key: &str) -> (String, Option<Uuid>, Option<String>) {
    sqlx::query_as(AssertSqlSafe(format!(
        "SELECT status::text, validated_by, review_note FROM {table} WHERE import_key = $1"
    )))
    .bind(key)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn validate(pool: &PgPool, table: &str, key: &str, by: Uuid, note: Option<&str>) {
    sqlx::query(AssertSqlSafe(format!(
        "UPDATE {table} SET status = 'validated', validated_by = $2, validated_at = now(), \
         review_note = $3 WHERE import_key = $1"
    )))
    .bind(key)
    .bind(by)
    .bind(note)
    .execute(pool)
    .await
    .unwrap();
}

async fn count(pool: &PgPool, table: &str) -> usize {
    let count: i64 = sqlx::query_scalar(AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
        .fetch_one(pool)
        .await
        .unwrap();
    usize::try_from(count).unwrap()
}

/// The cards of the lesson imported under `lesson`, in order: their key, or
/// their Arabic when written in the admin area.
async fn cards_of(pool: &PgPool, lesson: &str) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT coalesce(cards.import_key, cards.arabic) FROM cards \
         INNER JOIN lessons ON cards.lesson_id = lessons.id \
         WHERE lessons.import_key = $1 ORDER BY cards.position",
    )
    .bind(lesson)
    .fetch_all(pool)
    .await
    .unwrap()
}

fn keys(lesson: &str, cards: &[&str]) -> Vec<String> {
    cards
        .iter()
        .map(|card| format!("arabe/salutations/{lesson}/{card}"))
        .collect()
}

fn tally(created: usize, updated: usize, moved: usize, deleted: usize, unchanged: usize) -> Tally {
    Tally {
        created,
        updated,
        moved,
        deleted,
        unchanged,
    }
}

fn all_unchanged((units, lessons, cards): (usize, usize, usize)) -> SyncReport {
    SyncReport {
        units: tally(0, 0, 0, 0, units),
        lessons: tally(0, 0, 0, 0, lessons),
        cards: tally(0, 0, 0, 0, cards),
    }
}

async fn super_admin(pool: &PgPool) -> Uuid {
    ensure_super_admin(pool, "admin", PASSWORD.to_owned())
        .await
        .unwrap()
}

/// Syncs [`SALUTATIONS`] into a database with a super admin, whose id it
/// returns.
async fn synced(pool: &PgPool, store: &MemoryStore) -> Uuid {
    let author = super_admin(pool).await;
    sync_salutations(pool, store, SALUTATIONS).await.unwrap();
    author
}

fn memory() -> Storage {
    Storage::Memory(MemoryStore::default())
}

/// [`SALUTATIONS`] without its second lesson.
fn without_politesse() -> String {
    SALUTATIONS
        .split("  - key: politesse")
        .next()
        .unwrap()
        .to_owned()
}

#[sqlx::test]
async fn the_first_sync_creates_the_content_files_to_review(pool: PgPool) {
    let author = super_admin(&pool).await;

    let report = sync_content(&pool, &memory()).await.unwrap();

    let (units, lessons, cards) = Catalogue::embedded().unwrap().counts();
    assert_eq!(
        report,
        SyncReport {
            units: tally(units, 0, 0, 0, 0),
            lessons: tally(lessons, 0, 0, 0, 0),
            cards: tally(cards, 0, 0, 0, 0),
        }
    );
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
    assert_eq!(count(&pool, "units").await, units);
    assert_eq!(count(&pool, "lessons").await, lessons);
    assert_eq!(count(&pool, "cards").await, cards);

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
async fn a_second_sync_changes_nothing(pool: PgPool) {
    super_admin(&pool).await;
    sync_content(&pool, &memory()).await.unwrap();

    let report = sync_content(&pool, &memory()).await.unwrap();

    let counts = Catalogue::embedded().unwrap().counts();
    assert_eq!(report, all_unchanged(counts));
    assert_eq!(count(&pool, "cards").await, counts.2);
}

#[sqlx::test]
async fn a_text_changed_in_the_files_goes_back_to_review_without_its_note(pool: PgPool) {
    let store = MemoryStore::default();
    let author = synced(&pool, &store).await;
    let salam = "arabe/salutations/bonjour/salam";
    let sbah = "arabe/salutations/bonjour/sbah";
    validate(&pool, "cards", salam, author, Some("Il manque le tanwin")).await;
    validate(&pool, "cards", sbah, author, Some("Rien à redire")).await;
    validate(&pool, "lessons", "arabe/salutations/bonjour", author, None).await;
    let before = card(&pool, salam).await;

    let corrected = SALUTATIONS.replace("arabic: سَلَام\n", "arabic: سَلَامٌ\n");
    let report = sync_salutations(&pool, &store, &corrected).await.unwrap();

    assert_eq!(report.cards, tally(0, 1, 0, 0, 3));
    assert_eq!(report.lessons, tally(0, 0, 0, 0, 2));
    let after = card(&pool, salam).await;
    assert_eq!((after.id, after.arabic.as_str()), (before.id, "سَلَامٌ"));
    assert_eq!(after.position, 1);
    assert_eq!(
        review(&pool, "cards", salam).await,
        ("to_review".to_owned(), None, None)
    );
    assert_eq!(
        review(&pool, "cards", sbah).await,
        (
            "validated".to_owned(),
            Some(author),
            Some("Rien à redire".to_owned())
        )
    );
    assert_eq!(
        review(&pool, "lessons", "arabe/salutations/bonjour").await,
        ("validated".to_owned(), Some(author), None)
    );

    let again = sync_salutations(&pool, &store, &corrected).await.unwrap();
    assert_eq!(again, all_unchanged((1, 2, 4)));
}

#[sqlx::test]
async fn a_move_in_the_files_keeps_the_status(pool: PgPool) {
    let store = MemoryStore::default();
    let author = synced(&pool, &store).await;
    let msa = "arabe/salutations/bonjour/msa";
    validate(&pool, "cards", msa, author, Some("Bien")).await;
    let id = card(&pool, msa).await.id;

    let salam = "      - key: salam\n        arabic: سَلَام\n        transliteration: salam\n        \
                 meaning_fr: Paix\n";
    let reordered = SALUTATIONS.replace(salam, "").replace(
        "        meaning_fr: Bonsoir\n",
        &format!("        meaning_fr: Bonsoir\n{salam}"),
    );
    let report = sync_salutations(&pool, &store, &reordered).await.unwrap();

    assert_eq!(report.cards, tally(0, 0, 1, 0, 3));
    assert_eq!(
        cards_of(&pool, "arabe/salutations/bonjour").await,
        keys("bonjour", &["sbah", "msa", "salam"])
    );
    assert_eq!(card(&pool, msa).await.id, id);
    assert_eq!(
        review(&pool, "cards", msa).await,
        (
            "validated".to_owned(),
            Some(author),
            Some("Bien".to_owned())
        )
    );
}

#[sqlx::test]
async fn puts_back_a_row_moved_elsewhere_and_closes_the_gap(pool: PgPool) {
    let store = MemoryStore::default();
    let author = synced(&pool, &store).await;
    let choukran = "arabe/salutations/politesse/choukran";
    let bonjour = imported_id(&pool, "lessons", "arabe/salutations/bonjour").await;
    sqlx::query(
        "UPDATE cards SET position = position + 1 \
         WHERE lesson_id = $1 AND position >= 2",
    )
    .bind(bonjour)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE cards SET lesson_id = $1, position = 2 WHERE import_key = $2")
        .bind(bonjour)
        .bind(choukran)
        .execute(&pool)
        .await
        .unwrap();
    validate(&pool, "cards", choukran, author, None).await;

    let report = sync_salutations(&pool, &store, SALUTATIONS).await.unwrap();

    assert_eq!(report.cards, tally(0, 0, 1, 0, 3));
    assert_eq!(
        cards_of(&pool, "arabe/salutations/bonjour").await,
        keys("bonjour", &["salam", "sbah", "msa"])
    );
    assert_eq!(
        cards_of(&pool, "arabe/salutations/politesse").await,
        keys("politesse", &["choukran"])
    );
    assert_eq!(review(&pool, "cards", choukran).await.0, "validated");
}

/// Stores a recording for the row of `table` imported under `key`, and
/// returns the key of its object.
async fn record(pool: &PgPool, store: &MemoryStore, table: &str, key: &str) -> String {
    let audio = Uuid::new_v4();
    let object = format!("audio/{audio}");
    store
        .put(&object, "audio/wav", bytes::Bytes::from_static(b"RIFF"))
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO audio (id, object_key, content_type, size_bytes) \
         VALUES ($1, $2, 'audio/wav', 4)",
    )
    .bind(audio)
    .bind(&object)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(AssertSqlSafe(format!(
        "UPDATE {table} SET audio_id = $1 WHERE import_key = $2"
    )))
    .bind(audio)
    .bind(key)
    .execute(pool)
    .await
    .unwrap();
    object
}

#[sqlx::test]
async fn deletes_what_is_gone_from_the_files_with_its_recordings(pool: PgPool) {
    let store = MemoryStore::default();
    synced(&pool, &store).await;
    record(&pool, &store, "cards", "arabe/salutations/bonjour/sbah").await;
    record(
        &pool,
        &store,
        "cards",
        "arabe/salutations/politesse/choukran",
    )
    .await;
    record(&pool, &store, "lessons", "arabe/salutations/politesse").await;
    let kept = record(&pool, &store, "cards", "arabe/salutations/bonjour/salam").await;

    let without = without_politesse().replace(
        "      - key: sbah\n        arabic: صْبَاح الْخِير\n        transliteration: sbah el \
         khir\n        meaning_fr: Bonjour, le matin\n",
        "",
    );
    let report = sync_salutations(&pool, &store, &without).await.unwrap();

    assert_eq!(report.cards, tally(0, 0, 0, 2, 2));
    assert_eq!(report.lessons, tally(0, 0, 0, 1, 1));
    assert_eq!(
        cards_of(&pool, "arabe/salutations/bonjour").await,
        keys("bonjour", &["salam", "msa"])
    );
    assert_eq!(count(&pool, "lessons").await, 1);
    assert_eq!(count(&pool, "audio").await, 1);
    assert_eq!(store.keys(), vec![kept]);
}

#[sqlx::test]
async fn keeps_handwritten_cards_after_the_imported_card_they_followed(pool: PgPool) {
    let store = MemoryStore::default();
    synced(&pool, &store).await;
    create_account(&pool, "relectrice", "admin").await;
    let server = Server::new(&pool);
    let cookie = server.session("relectrice").await;
    let bonjour = imported_id(&pool, "lessons", "arabe/salutations/bonjour").await;
    let answer = server
        .send(
            Method::POST,
            "/api/v1/cards",
            Some(&cookie),
            Some(&json!({
                "lessonId": bonjour,
                "arabic": "صْبَاح النُّور",
                "variety": "darija",
                "position": 3,
            })),
        )
        .await;
    assert_eq!(answer.status, StatusCode::CREATED, "{}", answer.body);

    let reordered = SALUTATIONS
        .replace("key: salam", "key: tmp")
        .replace("key: msa", "key: salam")
        .replace("key: tmp", "key: msa");
    sync_salutations(&pool, &store, &reordered).await.unwrap();

    let mut expected = keys("bonjour", &["msa", "sbah"]);
    expected.push("صْبَاح النُّور".to_owned());
    expected.extend(keys("bonjour", &["salam"]));
    assert_eq!(cards_of(&pool, "arabe/salutations/bonjour").await, expected);
}

#[sqlx::test]
async fn refuses_to_delete_an_entry_holding_handwritten_content(pool: PgPool) {
    let store = MemoryStore::default();
    synced(&pool, &store).await;
    let politesse = imported_id(&pool, "lessons", "arabe/salutations/politesse").await;
    let handwritten: Uuid = sqlx::query_scalar(
        "INSERT INTO cards (lesson_id, position, arabic, variety) \
         VALUES ($1, 2, 'مَرْحَبًا', 'darija') RETURNING id",
    )
    .bind(politesse)
    .fetch_one(&pool)
    .await
    .unwrap();

    let without = without_politesse().replace("arabic: سَلَام\n", "arabic: سَلَامٌ\n");
    let error = sync_salutations(&pool, &store, &without).await.unwrap_err();

    let held = error
        .downcast_ref::<HeldByRemovedEntries>()
        .unwrap_or_else(|| panic!("{error:#}"));
    assert_eq!(held.0.len(), 1, "{held}");
    assert!(
        held.0[0].starts_with("arabe/salutations/politesse (lesson) holds « مَرْحَبًا »"),
        "{held}"
    );
    assert!(held.0[0].contains(&handwritten.to_string()), "{held}");
    assert_eq!(count(&pool, "lessons").await, 2);
    assert_eq!(
        card(&pool, "arabe/salutations/bonjour/salam").await.arabic,
        "سَلَام"
    );
}

#[sqlx::test]
async fn refuses_to_delete_a_unit_holding_a_handwritten_lesson(pool: PgPool) {
    let store = MemoryStore::default();
    synced(&pool, &store).await;
    let unit = imported_id(&pool, "units", "arabe/salutations").await;
    sqlx::query("INSERT INTO lessons (unit_id, position, title) VALUES ($1, 3, 'Mes ajouts')")
        .bind(unit)
        .execute(&pool)
        .await
        .unwrap();

    let other = SALUTATIONS.replace("key: salutations", "key: accueil");
    let error = sync_files(&pool, &store, &[("arabe/01-accueil.yaml", &other)])
        .await
        .unwrap_err();

    let held = error.downcast_ref::<HeldByRemovedEntries>().unwrap();
    assert_eq!(held.0.len(), 1, "{held}");
    assert!(
        held.0[0].starts_with("arabe/salutations (unit) holds « Mes ajouts »"),
        "{held}"
    );
    assert_eq!(count(&pool, "units").await, 1);
}

#[sqlx::test]
async fn two_syncs_at_once_create_everything_once(pool: PgPool) {
    super_admin(&pool).await;
    let storage = memory();

    let (first, second) =
        tokio::join!(sync_content(&pool, &storage), sync_content(&pool, &storage));

    let (first, second) = (first.unwrap(), second.unwrap());
    let counts = Catalogue::embedded().unwrap().counts();
    assert_eq!(first.cards.created + second.cards.created, counts.2);
    assert!(
        first == all_unchanged(counts) || second == all_unchanged(counts),
        "{first:?} {second:?}"
    );
    assert_eq!(count(&pool, "units").await, counts.0);
    assert_eq!(count(&pool, "lessons").await, counts.1);
    assert_eq!(count(&pool, "cards").await, counts.2);
}

#[sqlx::test]
async fn refuses_to_sync_without_a_super_admin(pool: PgPool) {
    let error = sync_content(&pool, &memory()).await.unwrap_err();

    assert!(error.to_string().contains("no super admin"), "{error:#}");
    assert_eq!(count(&pool, "units").await, 0);
}
