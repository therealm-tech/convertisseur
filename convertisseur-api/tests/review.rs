//! The review queue: lessons with how their cards stand, a lesson's cards,
//! and their validation in one go.

mod common;

use axum::http::{Method, StatusCode};
use common::{
    Answer, PASSWORD, SALUTATIONS, Server, create_account, imported_id, sync_salutations,
};
use convertisseur_api::{
    ensure_super_admin,
    storage::{MemoryStore, Storage},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

/// [`SALUTATIONS`] synced, every row to review, and a reviewer logged in.
struct Fixture {
    pool: PgPool,
    server: Server,
    admin: String,
    unit: Uuid,
    bonjour: Uuid,
    politesse: Uuid,
    salam: Uuid,
    sbah: Uuid,
    msa: Uuid,
    choukran: Uuid,
}

impl Fixture {
    async fn new(pool: &PgPool) -> Self {
        let store = MemoryStore::default();
        ensure_super_admin(pool, "admin", PASSWORD.to_owned())
            .await
            .unwrap();
        sync_salutations(pool, &store, SALUTATIONS).await.unwrap();
        create_account(pool, "relectrice", "admin").await;
        let server = Server::with_storage(pool, Storage::Memory(store));
        let admin = server.session("relectrice").await;
        let card =
            async |key| imported_id(pool, "cards", &format!("arabe/salutations/{key}")).await;
        Self {
            pool: pool.clone(),
            unit: imported_id(pool, "units", "arabe/salutations").await,
            bonjour: imported_id(pool, "lessons", "arabe/salutations/bonjour").await,
            politesse: imported_id(pool, "lessons", "arabe/salutations/politesse").await,
            salam: card("bonjour/salam").await,
            sbah: card("bonjour/sbah").await,
            msa: card("bonjour/msa").await,
            choukran: card("politesse/choukran").await,
            server,
            admin,
        }
    }

    async fn send(&self, method: Method, uri: &str, body: Option<&Value>) -> Answer {
        self.server.send(method, uri, Some(&self.admin), body).await
    }

    async fn get(&self, uri: &str) -> Value {
        let answer = self.send(Method::GET, uri, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        answer.body
    }

    async fn validate(&self, collection: &str, id: Uuid) {
        let answer = self
            .send(
                Method::PUT,
                &format!("/api/v1/{collection}/{id}/status"),
                Some(&json!({ "status": "validated" })),
            )
            .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    }

    async fn flag(&self, card: Uuid, note: &str) {
        let answer = self
            .send(
                Method::PUT,
                &format!("/api/v1/cards/{card}/review-note"),
                Some(&json!({ "text": note })),
            )
            .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    }

    async fn validate_in_bulk(&self, lesson: Uuid, cards: &[Uuid]) -> Answer {
        self.send(
            Method::POST,
            &format!("/api/v1/lessons/{lesson}/card-validations"),
            Some(&json!({ "cardIds": cards })),
        )
        .await
    }

    /// Gives the card a recording, as an upload would.
    async fn record(&self, card: Uuid) {
        let audio = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO audio (id, object_key, content_type, size_bytes) \
             VALUES ($1, 'audio/' || $1::text, 'audio/wav', 44)",
        )
        .bind(audio)
        .execute(&self.pool)
        .await
        .unwrap();
        sqlx::query("UPDATE cards SET audio_id = $2 WHERE id = $1")
            .bind(card)
            .bind(audio)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    async fn status_of(&self, card: Uuid) -> String {
        sqlx::query_scalar("SELECT status::text FROM cards WHERE id = $1")
            .bind(card)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }
}

/// The titles of the lessons, in order.
fn titles(page: &Value) -> Vec<&str> {
    page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|lesson| lesson["title"].as_str().unwrap())
        .collect()
}

/// The card counts of the lesson titled `title` on the page.
fn counts<'a>(page: &'a Value, title: &str) -> &'a Value {
    let lesson = page["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lesson| lesson["title"] == title)
        .unwrap_or_else(|| panic!("{title} is not listed: {page}"));
    &lesson["cardCounts"]
}

#[sqlx::test]
async fn lists_what_is_left_to_review_with_its_counts(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;

    let queue = fixture.get("/api/v1/lessons?pendingReview=true").await;
    assert_eq!(titles(&queue), ["Bonjour", "Politesse"]);
    assert_eq!(queue["total"], 2);
    assert_eq!(queue["itemsToReview"], 7, "1 unit, 2 lessons, 4 cards");
    assert_eq!(queue["cardsWithoutAudio"], 4);
    assert_eq!(
        counts(&queue, "Bonjour"),
        &json!({ "validated": 0, "flagged": 0, "toReview": 3, "withoutAudio": 3 })
    );
    let bonjour = &queue["items"][0];
    assert_eq!(bonjour["id"], fixture.bonjour.to_string());
    assert_eq!(bonjour["status"], "to_review");
    assert_eq!(bonjour["hasAudio"], false);
    assert_eq!(
        bonjour["unit"],
        json!({
            "id": fixture.unit,
            "track": "arabe",
            "position": 1,
            "title": "Salutations",
            "status": "to_review",
            "reviewNote": null,
        })
    );

    fixture.validate("units", fixture.unit).await;
    fixture.validate("lessons", fixture.politesse).await;
    fixture.validate("cards", fixture.choukran).await;
    fixture.validate("cards", fixture.salam).await;
    fixture
        .flag(fixture.sbah, "Le kasra du 2e mot manque")
        .await;

    let queue = fixture.get("/api/v1/lessons?pendingReview=true").await;
    assert_eq!(titles(&queue), ["Bonjour"], "Politesse has nothing left");
    assert_eq!(
        counts(&queue, "Bonjour"),
        &json!({ "validated": 1, "flagged": 1, "toReview": 1, "withoutAudio": 3 })
    );
    assert_eq!(
        queue["itemsToReview"], 3,
        "the lesson Bonjour, sbah flagged and msa"
    );

    let unit = fixture
        .get(&format!("/api/v1/lessons?unitId={}", fixture.unit))
        .await;
    assert_eq!(titles(&unit), ["Bonjour", "Politesse"], "whatever is left");
    assert_eq!(
        counts(&unit, "Politesse"),
        &json!({ "validated": 1, "flagged": 0, "toReview": 0, "withoutAudio": 1 })
    );
}

#[sqlx::test]
async fn pages_the_lessons(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;

    let page = fixture.get("/api/v1/lessons?perPage=1&page=2").await;
    assert_eq!(titles(&page), ["Politesse"]);
    assert_eq!(
        (page["page"].clone(), page["perPage"].clone()),
        (json!(2), json!(1))
    );
    assert_eq!(page["total"], 2);

    let past = fixture.get("/api/v1/lessons?perPage=1&page=3").await;
    assert_eq!(past["items"], json!([]));
    assert_eq!(past["total"], 2);

    fixture
        .send(Method::GET, "/api/v1/lessons?perPage=101", None)
        .await
        .assert_invalid("perPage", "range");
}

#[sqlx::test]
async fn filters_the_lessons_with_cards_without_audio(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    fixture.record(fixture.choukran).await;
    fixture.record(fixture.salam).await;

    let silent = fixture.get("/api/v1/lessons?missingAudio=true").await;
    assert_eq!(
        titles(&silent),
        ["Bonjour"],
        "Politesse's only card has a recording, and a lesson's own is optional"
    );
    assert_eq!(counts(&silent, "Bonjour")["withoutAudio"], 2);
    assert_eq!(silent["items"][0]["hasAudio"], false);
    assert_eq!(silent["cardsWithoutAudio"], 2);

    for (collection, id) in [
        ("units", fixture.unit),
        ("lessons", fixture.bonjour),
        ("lessons", fixture.politesse),
        ("cards", fixture.salam),
        ("cards", fixture.sbah),
        ("cards", fixture.msa),
        ("cards", fixture.choukran),
    ] {
        fixture.validate(collection, id).await;
    }

    let queue = fixture.get("/api/v1/lessons?pendingReview=true").await;
    assert_eq!(queue["items"], json!([]));
    assert_eq!(queue["itemsToReview"], 0);
    let silent = fixture.get("/api/v1/lessons?missingAudio=true").await;
    assert_eq!(
        titles(&silent),
        ["Bonjour"],
        "validated, still without audio"
    );
}

#[sqlx::test]
async fn lists_a_lessons_cards_in_order(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    fixture
        .flag(fixture.sbah, "Le kasra du 2e mot manque")
        .await;

    let cards = fixture
        .get(&format!("/api/v1/lessons/{}/cards", fixture.bonjour))
        .await;
    let arabic: Vec<&str> = cards["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|card| card["arabic"].as_str().unwrap())
        .collect();
    assert_eq!(arabic, ["سَلَام", "صْبَاح الْخِير", "مْسَا الْخِير"]);
    assert_eq!(cards["total"], 3);
    let sbah = &cards["items"][1];
    assert_eq!(sbah["reviewNote"], "Le kasra du 2e mot manque");
    assert_eq!(sbah["importKey"], "arabe/salutations/bonjour/sbah");
    assert_eq!(sbah["createdBy"]["username"], "admin");
    assert_eq!(sbah["audio"], Value::Null);

    let second = fixture
        .get(&format!(
            "/api/v1/lessons/{}/cards?perPage=2&page=2",
            fixture.bonjour
        ))
        .await;
    assert_eq!(second["items"][0]["transliteration"], "msa el khir");
    assert_eq!(second["total"], 3);

    fixture
        .send(
            Method::GET,
            &format!("/api/v1/lessons/{}/cards", Uuid::new_v4()),
            None,
        )
        .await
        .assert_problem(StatusCode::NOT_FOUND, "not-found");
}

#[sqlx::test]
async fn validates_the_cards_that_are_not_flagged(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    fixture
        .flag(fixture.sbah, "Le kasra du 2e mot manque")
        .await;
    fixture.validate("cards", fixture.msa).await;
    let msa_validated_at: Value =
        fixture.get(&format!("/api/v1/cards/{}", fixture.msa)).await["validatedAt"].clone();

    let answer = fixture
        .validate_in_bulk(fixture.bonjour, &[fixture.msa, fixture.sbah, fixture.salam])
        .await;

    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    let cards = answer.body["cards"].as_array().unwrap();
    let ids: Vec<&str> = cards.iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        [fixture.salam, fixture.sbah, fixture.msa].map(|id| id.to_string()),
        "in the lesson's order"
    );
    assert_eq!(cards[0]["status"], "validated");
    assert_eq!(cards[0]["validatedBy"]["username"], "relectrice");
    assert_eq!(cards[1]["status"], "to_review", "flagged, left alone");
    assert_eq!(cards[1]["reviewNote"], "Le kasra du 2e mot manque");
    assert_eq!(cards[1]["validatedAt"], Value::Null);
    assert_eq!(cards[2]["validatedAt"], msa_validated_at, "kept as it was");

    let queue = fixture
        .get(&format!("/api/v1/lessons?unitId={}", fixture.unit))
        .await;
    assert_eq!(
        counts(&queue, "Bonjour"),
        &json!({ "validated": 2, "flagged": 1, "toReview": 0, "withoutAudio": 3 })
    );
}

#[sqlx::test]
async fn validates_in_bulk_all_or_nothing(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;

    for strangers in [
        vec![fixture.salam, fixture.choukran],
        vec![fixture.salam, Uuid::new_v4()],
    ] {
        fixture
            .validate_in_bulk(fixture.bonjour, &strangers)
            .await
            .assert_invalid("cardIds", "card_not_in_lesson");
        assert_eq!(fixture.status_of(fixture.salam).await, "to_review");
    }

    fixture
        .validate_in_bulk(fixture.bonjour, &[])
        .await
        .assert_invalid("cardIds", "length");
    fixture
        .validate_in_bulk(Uuid::new_v4(), &[fixture.salam])
        .await
        .assert_problem(StatusCode::NOT_FOUND, "not-found");

    let answer = fixture
        .validate_in_bulk(fixture.bonjour, &[fixture.salam, fixture.salam])
        .await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(answer.body["cards"].as_array().unwrap().len(), 1);
    assert_eq!(fixture.status_of(fixture.salam).await, "validated");
}

#[sqlx::test]
async fn the_queue_is_for_admins_only(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    create_account(&pool, "aicha", "user").await;
    let user = fixture.server.session("aicha").await;
    let body = json!({ "cardIds": [fixture.salam] });

    for (method, uri, body) in [
        (
            Method::GET,
            "/api/v1/lessons?pendingReview=true".to_owned(),
            None,
        ),
        (
            Method::GET,
            format!("/api/v1/lessons/{}/cards", fixture.bonjour),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/lessons/{}/card-validations", fixture.bonjour),
            Some(&body),
        ),
    ] {
        fixture
            .server
            .send(method.clone(), &uri, Some(&user), body)
            .await
            .assert_problem(StatusCode::FORBIDDEN, "forbidden");
        fixture
            .server
            .send(method, &uri, None, body)
            .await
            .assert_problem(StatusCode::UNAUTHORIZED, "unauthenticated");
    }
    assert_eq!(fixture.status_of(fixture.salam).await, "to_review");
}

#[sqlx::test]
async fn documents_the_review_endpoints(pool: PgPool) {
    let server = Server::new(&pool);
    let document = server.send(Method::GET, "/openapi.json", None, None).await;

    for (path, method) in [
        ("/api/v1/lessons", "get"),
        ("/api/v1/lessons", "post"),
        ("/api/v1/lessons/{lessonId}/cards", "get"),
        ("/api/v1/lessons/{lessonId}/card-validations", "post"),
    ] {
        let operation = &document.body["paths"][path][method];
        assert_eq!(operation["tags"], json!(["Content"]), "{method} {path}");
        assert!(
            operation["security"].is_array(),
            "{method} {path} needs a session"
        );
    }
}
