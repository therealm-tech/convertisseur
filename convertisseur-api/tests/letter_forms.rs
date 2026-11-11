//! The initial, medial and final forms of the letters: written by the
//! content files alone, kept in step by the sync, and served to learners
//! and staff.

mod common;

use axum::http::{Method, StatusCode};
use common::{PASSWORD, Server, create_account, imported_id, sync_files};
use convertisseur_api::{
    ensure_super_admin,
    storage::{MemoryStore, Storage},
    sync::Catalogue,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const LETTERS: &str = "
key: alphabet
title: L'alphabet
variety: classique
lessons:
  - key: lettres
    title: Quelques lettres
    cards:
      - key: ba
        arabic: ب
        forms:
          initial: بـ
          medial: ـبـ
          final: ـب
        transliteration: bāʾ
      - key: dal
        arabic: د
        forms:
          final: ـد
        transliteration: dāl
      - key: ta-marbuta
        arabic: ة
        forms:
          final: ـة
        transliteration: tāʾ marbūṭa
      - key: salam
        arabic: سَلَام
        transliteration: salām
";

const BA: &str = "arabe/alphabet/lettres/ba";

struct Fixture {
    server: Server,
    store: MemoryStore,
    admin: String,
    author: Uuid,
}

impl Fixture {
    /// [`LETTERS`] synced, and an admin logged in.
    async fn new(pool: &PgPool) -> Self {
        let store = MemoryStore::default();
        let author = ensure_super_admin(pool, "admin", PASSWORD.to_owned())
            .await
            .unwrap();
        sync_files(pool, &store, &[("arabe/01-alphabet.yaml", LETTERS)])
            .await
            .unwrap();
        create_account(pool, "relectrice", "admin").await;
        let server = Server::with_storage(pool, Storage::Memory(store.clone()));
        let admin = server.session("relectrice").await;
        Self {
            server,
            store,
            admin,
            author,
        }
    }

    async fn get(&self, uri: &str) -> Value {
        let answer = self
            .server
            .send(Method::GET, uri, Some(&self.admin), None)
            .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        answer.body
    }

    async fn card(&self, pool: &PgPool, key: &str) -> Value {
        let id = imported_id(pool, "cards", &format!("arabe/alphabet/lettres/{key}")).await;
        self.get(&format!("/api/v1/cards/{id}")).await
    }
}

async fn validate_everything(pool: &PgPool, by: Uuid) {
    for table in ["units", "lessons", "cards"] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "UPDATE {table} SET status = 'validated', validated_by = $1, validated_at = now()"
        )))
        .bind(by)
        .execute(pool)
        .await
        .unwrap();
    }
}

#[sqlx::test]
async fn staff_see_the_forms_and_their_kind(pool: PgPool) {
    let f = Fixture::new(&pool).await;

    assert_eq!(
        f.card(&pool, "ba").await["forms"],
        json!({ "kind": "joining", "initial": "بـ", "medial": "ـبـ", "final": "ـب" })
    );
    assert_eq!(
        f.card(&pool, "dal").await["forms"],
        json!({ "kind": "non_joining", "final": "ـد" })
    );
    assert_eq!(
        f.card(&pool, "ta-marbuta").await["forms"],
        json!({ "kind": "word_final", "final": "ـة" })
    );
    assert_eq!(f.card(&pool, "salam").await["forms"], Value::Null);
}

#[sqlx::test]
async fn a_card_written_in_the_admin_area_has_no_forms(pool: PgPool) {
    let f = Fixture::new(&pool).await;
    let lesson = imported_id(&pool, "lessons", "arabe/alphabet/lettres").await;

    let created = f
        .server
        .send(
            Method::POST,
            "/api/v1/cards",
            Some(&f.admin),
            Some(&json!({
                "lessonId": lesson, "arabic": "ت", "variety": "classique",
                "forms": { "kind": "joining", "initial": "تـ", "medial": "ـتـ", "final": "ـت" },
            })),
        )
        .await;

    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    assert_eq!(created.body["forms"], Value::Null);
}

#[sqlx::test]
async fn learners_see_the_forms_of_the_cards_they_study(pool: PgPool) {
    let f = Fixture::new(&pool).await;
    validate_everything(&pool, f.author).await;
    create_account(&pool, "eleve", "user").await;
    let learner = f.server.session("eleve").await;
    let lesson = imported_id(&pool, "lessons", "arabe/alphabet/lettres").await;

    let answer = f
        .server
        .send(
            Method::GET,
            &format!("/api/v1/tracks/arabe/lessons/{lesson}"),
            Some(&learner),
            None,
        )
        .await;

    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    let forms: Vec<&Value> = answer.body["cards"]
        .as_array()
        .unwrap()
        .iter()
        .map(|card| &card["forms"])
        .collect();
    assert_eq!(
        forms,
        [
            &json!({ "kind": "joining", "initial": "بـ", "medial": "ـبـ", "final": "ـب" }),
            &json!({ "kind": "non_joining", "final": "ـد" }),
            &json!({ "kind": "word_final", "final": "ـة" }),
            &Value::Null,
        ]
    );
}

#[sqlx::test]
async fn a_form_changed_in_the_files_sends_the_card_back_to_review(pool: PgPool) {
    let f = Fixture::new(&pool).await;
    validate_everything(&pool, f.author).await;
    let before = f.card(&pool, "ba").await;
    assert_eq!(before["status"], "validated");

    let without_forms = LETTERS.replace(
        "        forms:\n          initial: بـ\n          medial: ـبـ\n          final: ـب\n",
        "",
    );
    let report = sync_files(
        &pool,
        &f.store,
        &[("arabe/01-alphabet.yaml", &without_forms)],
    )
    .await
    .unwrap();

    assert_eq!((report.cards.updated, report.cards.unchanged), (1, 3));
    let after = f.card(&pool, "ba").await;
    assert_eq!(after["id"], before["id"]);
    assert_eq!(after["forms"], Value::Null);
    assert_eq!(after["status"], "to_review");
    assert_eq!(f.card(&pool, "dal").await["status"], "validated");

    let report = sync_files(&pool, &f.store, &[("arabe/01-alphabet.yaml", LETTERS)])
        .await
        .unwrap();
    assert_eq!((report.cards.updated, report.cards.unchanged), (1, 3));
    assert_eq!(
        f.card(&pool, "ba").await["forms"],
        json!({ "kind": "joining", "initial": "بـ", "medial": "ـبـ", "final": "ـب" })
    );

    let again = sync_files(&pool, &f.store, &[("arabe/01-alphabet.yaml", LETTERS)])
        .await
        .unwrap();
    assert_eq!((again.cards.updated, again.cards.unchanged), (0, 4));
}

#[test]
fn the_sync_refuses_invalid_forms() {
    let invalid = [
        LETTERS.replace(
            "          final: ـد\n",
            "          initial: دـ\n          final: ـد\n",
        ),
        LETTERS.replace("          initial: بـ\n", ""),
        LETTERS.replace(
            "        transliteration: salām\n",
            "        transliteration: salām\n        forms:\n          final: ـسلام\n",
        ),
        LETTERS.replace("final: ـة", "final: ـت"),
        LETTERS.replace("final: ـب\n", "final: بـ\n"),
    ];
    for text in &invalid {
        assert!(
            Catalogue::parse(&[("arabe/01-alphabet.yaml", text)]).is_err(),
            "accepted:\n{text}"
        );
    }
}

#[sqlx::test]
async fn the_table_refuses_a_medial_form_without_an_initial_one(pool: PgPool) {
    let f = Fixture::new(&pool).await;
    let id = imported_id(&pool, "cards", BA).await;

    let refused = sqlx::query("UPDATE cards SET initial_form = NULL WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap_err();
    assert!(
        refused.to_string().contains("cards_forms_check"),
        "{refused}"
    );
    let refused = sqlx::query(
        "UPDATE cards SET final_form = NULL, initial_form = NULL, medial_form = NULL \
         WHERE id = $1",
    )
    .bind(id)
    .execute(&pool)
    .await;
    assert!(refused.is_ok(), "no forms at all is a card without forms");
    assert_eq!(f.card(&pool, "ba").await["forms"], Value::Null);
}

#[sqlx::test]
async fn documents_the_forms(pool: PgPool) {
    let server = Server::new(&pool);

    let document = server.send(Method::GET, "/openapi.json", None, None).await;

    let schemas = &document.body["components"]["schemas"];
    let forms = &schemas["LetterFormsResponse"];
    let kinds: Vec<&Value> = forms["oneOf"]
        .as_array()
        .unwrap_or_else(|| panic!("{forms}"))
        .iter()
        .map(|variant| &variant["properties"]["kind"]["const"])
        .collect();
    assert_eq!(
        kinds,
        [
            &json!("joining"),
            &json!("non_joining"),
            &json!("word_final")
        ]
    );
    for dto in ["CardResponse", "StudyCardResponse"] {
        let property = schemas[dto]["properties"]["forms"].to_string();
        assert!(
            property.contains("LetterFormsResponse"),
            "{dto}: {property}"
        );
    }
}
