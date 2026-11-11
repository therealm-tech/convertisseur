//! What the admin area may and may not do to content imported from the
//! content files, and the review notes every item carries.

mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use common::{
    Answer, PASSWORD, SALUTATIONS, Server, create_account, imported_id, sync_salutations,
};
use convertisseur_api::{
    ensure_super_admin,
    storage::{MemoryStore, Storage},
};
use serde_json::{Value, json};
use sqlx::PgPool;

const BOUNDARY: &str = "imported-test-boundary";

struct Fixture {
    server: Server,
    admin: String,
    unit: String,
    lesson: String,
    card: String,
    card_lesson: String,
}

impl Fixture {
    /// [`SALUTATIONS`] synced, and an admin logged in.
    async fn new(pool: &PgPool) -> Self {
        let store = MemoryStore::default();
        ensure_super_admin(pool, "admin", PASSWORD.to_owned())
            .await
            .unwrap();
        sync_salutations(pool, &store, SALUTATIONS).await.unwrap();
        create_account(pool, "relectrice", "admin").await;
        let server = Server::with_storage(pool, Storage::Memory(store));
        let admin = server.session("relectrice").await;
        let unit = imported_id(pool, "units", "arabe/salutations").await;
        let lesson = imported_id(pool, "lessons", "arabe/salutations/bonjour").await;
        let card = imported_id(pool, "cards", "arabe/salutations/bonjour/sbah").await;
        Self {
            server,
            admin,
            unit: format!("/api/v1/units/{unit}"),
            lesson: format!("/api/v1/lessons/{lesson}"),
            card: format!("/api/v1/cards/{card}"),
            card_lesson: lesson.to_string(),
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

    async fn note(&self, item: &str, text: Value) -> Answer {
        self.send(
            Method::PUT,
            &format!("{item}/review-note"),
            Some(&json!({ "text": text })),
        )
        .await
    }
}

/// A 1 ms, 8 kHz, 8-bit mono WAV file.
fn wav() -> Vec<u8> {
    let data = 8_u32;
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&(36 + data).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    for word in [16_u32, 0x0001_0001, 8_000, 8_000, 0x0008_0001] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data.to_le_bytes());
    bytes.extend_from_slice(&[128; 8]);
    bytes
}

#[sqlx::test]
async fn refuses_to_edit_or_delete_imported_content(pool: PgPool) {
    let f = Fixture::new(&pool).await;
    let unit_before = f.get(&f.unit).await;
    let card_before = f.get(&f.card).await;
    assert_eq!(unit_before["importKey"], "arabe/salutations");
    assert_eq!(card_before["importKey"], "arabe/salutations/bonjour/sbah");

    let edits = [
        (
            &f.unit,
            json!({ "track": "arabe", "title": "Bonjour !", "position": 1 }),
        ),
        (
            &f.unit,
            json!({ "track": "islam", "title": "Salutations", "position": 1 }),
        ),
        (
            &f.unit,
            json!({ "track": "arabe", "title": "Salutations", "position": 2 }),
        ),
        (
            &f.lesson,
            json!({ "unitId": unit_before["id"], "title": "Salut", "position": 1 }),
        ),
        (
            &f.lesson,
            json!({ "unitId": unit_before["id"], "title": "Bonjour", "position": 2 }),
        ),
        (
            &f.card,
            json!({
                "lessonId": f.card_lesson, "arabic": "صْبَاح الْخِيرْ", "transliteration": "sbah el khir",
                "meaningFr": "Bonjour, le matin", "variety": "darija", "position": 2,
            }),
        ),
        (
            &f.card,
            json!({
                "lessonId": f.card_lesson, "arabic": "صْبَاح الْخِير", "transliteration": "sbah el khir",
                "meaningFr": "Bonjour, le matin", "variety": "classique", "position": 2,
            }),
        ),
        (
            &f.card,
            json!({
                "lessonId": f.card_lesson, "arabic": "صْبَاح الْخِير", "transliteration": "sbah el khir",
                "meaningFr": "Bonjour, le matin", "variety": "darija", "position": 1,
            }),
        ),
    ];
    for (item, body) in edits {
        f.send(Method::PUT, item, Some(&body))
            .await
            .assert_problem(StatusCode::CONFLICT, "imported-content");
    }
    for item in [&f.card, &f.lesson, &f.unit] {
        f.send(Method::DELETE, item, None)
            .await
            .assert_problem(StatusCode::CONFLICT, "imported-content");
    }

    assert_eq!(f.get(&f.unit).await, unit_before);
    assert_eq!(f.get(&f.card).await, card_before);
}

#[sqlx::test]
async fn keeps_status_audio_note_and_new_children_open_on_imported_content(pool: PgPool) {
    let f = Fixture::new(&pool).await;

    for item in [&f.unit, &f.lesson, &f.card] {
        let answer = f
            .send(
                Method::PUT,
                &format!("{item}/status"),
                Some(&json!({ "status": "validated" })),
            )
            .await;
        assert_eq!(answer.status, StatusCode::OK, "{item}: {}", answer.body);
        let answer = f.note(item, json!("À revoir")).await;
        assert_eq!(answer.status, StatusCode::OK, "{item}: {}", answer.body);
    }

    let mut body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.wav\"\r\n\
         Content-Type: audio/wav\r\n\r\n"
    )
    .into_bytes();
    body.extend(wav());
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    let upload = Request::put(format!("{}/audio", f.card))
        .header(header::COOKIE, &f.admin)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(body))
        .unwrap();
    let answer = f.server.request(upload).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(f.get(&f.card).await["audio"]["contentType"], "audio/wav");
    let answer = f
        .send(Method::DELETE, &format!("{}/audio", f.card), None)
        .await;
    assert_eq!(answer.status, StatusCode::NO_CONTENT, "{}", answer.body);

    let created = f
        .send(
            Method::POST,
            "/api/v1/cards",
            Some(&json!({
                "lessonId": f.card_lesson,
                "arabic": "صْبَاح النُّور",
                "variety": "darija",
                "position": 1,
            })),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    assert_eq!(created.body["importKey"], Value::Null);
    let handwritten = format!("/api/v1/cards/{}", created.body["id"].as_str().unwrap());
    let edited = f
        .send(
            Method::PUT,
            &handwritten,
            Some(&json!({
                "lessonId": f.card_lesson,
                "arabic": "صْبَاح النُّورْ",
                "variety": "darija",
                "position": 4,
            })),
        )
        .await;
    assert_eq!(edited.status, StatusCode::OK, "{}", edited.body);
    assert_eq!(edited.body["position"], 4);
    let deleted = f.send(Method::DELETE, &handwritten, None).await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT, "{}", deleted.body);
}

#[sqlx::test]
async fn a_review_note_is_bounded_and_leaves_the_status_alone(pool: PgPool) {
    let f = Fixture::new(&pool).await;
    let answer = f
        .send(
            Method::PUT,
            &format!("{}/status", f.card),
            Some(&json!({ "status": "validated" })),
        )
        .await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

    let longest = "ب".repeat(2000);
    let answer = f.note(&f.card, json!(format!("  {longest}  "))).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(answer.body, json!({ "text": longest }));
    let card = f.get(&f.card).await;
    assert_eq!(card["reviewNote"], json!(longest));
    assert_eq!(card["status"], "validated");

    f.note(&f.card, json!("ب".repeat(2001)))
        .await
        .assert_invalid("text", "review_note_length");
    for cleared in [json!("   "), json!(null)] {
        let answer = f.note(&f.card, cleared).await;
        assert_eq!(answer.body, json!({ "text": null }));
    }
    assert_eq!(f.get(&f.card).await["reviewNote"], Value::Null);

    f.server
        .send(
            Method::PUT,
            "/api/v1/cards/00000000-0000-0000-0000-000000000000/review-note",
            Some(&f.admin),
            Some(&json!({ "text": "?" })),
        )
        .await
        .assert_problem(StatusCode::NOT_FOUND, "not-found");
}

#[sqlx::test]
async fn only_admins_write_review_notes(pool: PgPool) {
    let f = Fixture::new(&pool).await;
    create_account(&pool, "apprenant", "user").await;
    let learner = f.server.session("apprenant").await;

    f.server
        .send(
            Method::PUT,
            &format!("{}/review-note", f.lesson),
            Some(&learner),
            Some(&json!({ "text": "Faux" })),
        )
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");
    f.server
        .send(
            Method::PUT,
            &format!("{}/review-note", f.lesson),
            None,
            Some(&json!({ "text": "Faux" })),
        )
        .await
        .assert_problem(StatusCode::UNAUTHORIZED, "unauthenticated");
    assert_eq!(f.get(&f.lesson).await["reviewNote"], Value::Null);
}

#[sqlx::test]
async fn lists_the_items_with_a_review_note_and_their_import_keys(pool: PgPool) {
    let f = Fixture::new(&pool).await;
    f.note(&f.card, json!("Vocalisation fausse sur le 2e mot"))
        .await;
    f.note(&f.unit, json!("Titre à revoir")).await;
    let answer = f
        .send(
            Method::PUT,
            &format!("{}/status", f.unit),
            Some(&json!({ "status": "validated" })),
        )
        .await;
    assert_eq!(answer.status, StatusCode::OK);

    let all = f.get("/api/v1/content-items").await;
    assert_eq!(all["total"], 7);
    assert_eq!(all["reviewNoteCount"], 2);
    assert_eq!(
        all["statusCounts"],
        json!({ "toReview": 6, "validated": 1 })
    );

    let noted = f.get("/api/v1/content-items?hasReviewNote=true").await;
    assert_eq!(noted["total"], 2);
    assert_eq!(noted["reviewNoteCount"], 2);
    assert_eq!(
        noted["statusCounts"],
        json!({ "toReview": 1, "validated": 1 })
    );
    let mut items: Vec<(String, String)> = noted["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            (
                item["importKey"].as_str().unwrap().to_owned(),
                item["reviewNote"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    items.sort();
    assert_eq!(
        items,
        [
            ("arabe/salutations", "Titre à revoir"),
            (
                "arabe/salutations/bonjour/sbah",
                "Vocalisation fausse sur le 2e mot"
            ),
        ]
        .map(|(key, note)| (key.to_owned(), note.to_owned()))
    );

    let to_review = f
        .get("/api/v1/content-items?hasReviewNote=true&status=to_review")
        .await;
    assert_eq!(to_review["total"], 1);
    assert_eq!(to_review["reviewNoteCount"], 1);
    assert_eq!(
        to_review["items"][0]["importKey"],
        "arabe/salutations/bonjour/sbah"
    );

    let without = f.get("/api/v1/content-items?hasReviewNote=false").await;
    assert_eq!(without["total"], 5);
    assert!(
        without["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["reviewNote"].is_null())
    );
}
