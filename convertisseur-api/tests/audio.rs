mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use common::{Answer, Server, create_account};
use convertisseur_api::storage::{MemoryStore, Storage};
use serde_json::{Value, json};
use sqlx::PgPool;

const BOUNDARY: &str = "convertisseur-test-boundary";

/// A 1-second, 8 kHz, 8-bit mono WAV file: 8,044 bytes.
fn wav() -> Vec<u8> {
    let data = 8_000_u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&8_000_u32.to_le_bytes());
    bytes.extend_from_slice(&8_000_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&8_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data.to_le_bytes());
    bytes.extend((0..data).map(|i| u8::try_from(i % 251).unwrap()));
    bytes
}

fn multipart(part: &str, content_type: &str, file: &[u8]) -> Vec<u8> {
    let mut body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{part}\"; \
         filename=\"enregistrement\"\r\nContent-Type: {content_type}\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(file);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

/// A `durationMs` text part, to append to a body from [`multipart`] or to
/// put ahead of it.
fn duration_part(value: &str) -> Vec<u8> {
    format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"durationMs\"\r\n\r\n{value}\r\n"
    )
    .into_bytes()
}

/// `file` followed by a `durationMs` part.
fn multipart_with_duration(file: &[u8], duration: &str) -> Vec<u8> {
    let mut body = multipart("file", "audio/wav", file);
    body.truncate(body.len() - format!("--{BOUNDARY}--\r\n").len());
    body.extend(duration_part(duration));
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    body
}

/// A minimal MP4 audio file, whose length the server cannot read.
fn mp4() -> Vec<u8> {
    let mut bytes = b"\0\0\0\x18ftypM4A \0\0\0\0M4A mp42".to_vec();
    bytes.extend_from_slice(&[0; 64]);
    bytes
}

struct Fixture {
    server: Server,
    store: MemoryStore,
    admin: String,
    unit: String,
    card: String,
    lesson: String,
}

impl Fixture {
    async fn new(pool: &PgPool) -> Self {
        let store = MemoryStore::default();
        let server = Server::with_storage(pool, Storage::Memory(store.clone()));
        create_account(pool, "nour", "admin").await;
        let admin = server.session("nour").await;
        let unit = server
            .send(
                Method::POST,
                "/api/v1/units",
                Some(&admin),
                Some(&json!({ "track": "arabe", "title": "Salutations" })),
            )
            .await;
        let lesson = server
            .send(
                Method::POST,
                "/api/v1/lessons",
                Some(&admin),
                Some(&json!({ "unitId": unit.body["id"], "title": "En darija" })),
            )
            .await;
        let card = server
            .send(
                Method::POST,
                "/api/v1/cards",
                Some(&admin),
                Some(&json!({
                    "lessonId": lesson.body["id"],
                    "arabic": "واش راك؟",
                    "variety": "darija",
                })),
            )
            .await;
        Self {
            server,
            store,
            admin,
            unit: format!("/api/v1/units/{}", unit.body["id"].as_str().unwrap()),
            card: format!("/api/v1/cards/{}", card.body["id"].as_str().unwrap()),
            lesson: format!("/api/v1/lessons/{}", lesson.body["id"].as_str().unwrap()),
        }
    }

    async fn upload_as(
        &self,
        cookie: &str,
        item: &str,
        content_type: &str,
        body: Vec<u8>,
    ) -> Answer {
        let request = Request::put(format!("{item}/audio"))
            .header(header::COOKIE, cookie)
            .header(header::CONTENT_TYPE, content_type)
            .body(Body::from(body))
            .unwrap();
        self.server.request(request).await
    }

    async fn upload(&self, item: &str, file: &[u8]) -> Answer {
        self.upload_as(
            &self.admin,
            item,
            &format!("multipart/form-data; boundary={BOUNDARY}"),
            multipart("file", "audio/wav", file),
        )
        .await
    }

    async fn play(&self, cookie: Option<&str>, audio_id: &str, range: Option<&str>) -> Answer {
        let mut request = Request::get(format!("/api/v1/audio/{audio_id}"));
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        if let Some(range) = range {
            request = request.header(header::RANGE, range);
        }
        self.server
            .request(request.body(Body::empty()).unwrap())
            .await
    }

    async fn get(&self, uri: &str) -> Value {
        self.server
            .send(Method::GET, uri, Some(&self.admin), None)
            .await
            .body
    }

    async fn validate(&self, item: &str) {
        let answer = self
            .server
            .send(
                Method::PUT,
                &format!("{item}/status"),
                Some(&self.admin),
                Some(&json!({ "status": "validated" })),
            )
            .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    }
}

async fn audio_rows(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM audio")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn stores_a_recognised_file_whatever_the_client_calls_it(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    fixture.validate(&fixture.card).await;

    let answer = fixture
        .upload_as(
            &fixture.admin,
            &fixture.card,
            &format!("multipart/form-data; boundary={BOUNDARY}"),
            multipart("file", "text/plain", &wav()),
        )
        .await;

    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(answer.body["contentType"], "audio/wav");
    assert_eq!(answer.body["sizeBytes"], 8_044);
    assert_eq!(answer.body["durationMs"], 1_000);
    let card = fixture.get(&fixture.card).await;
    assert_eq!(card["audio"], answer.body);
    assert_eq!(card["status"], "to_review");
    assert_eq!(
        fixture.store.keys(),
        [format!("audio/{}", answer.body["id"].as_str().unwrap())]
    );
}

#[sqlx::test]
async fn refuses_files_that_are_not_audio(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let multipart_type = format!("multipart/form-data; boundary={BOUNDARY}");

    for file in [
        &b"<html><body>hello</body></html>"[..],
        b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR",
    ] {
        fixture
            .upload_as(
                &fixture.admin,
                &fixture.card,
                &multipart_type,
                multipart("file", "audio/mpeg", file),
            )
            .await
            .assert_problem(StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported-audio");
    }
    fixture
        .upload_as(&fixture.admin, &fixture.card, "audio/wav", wav())
        .await
        .assert_problem(StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported-media-type");
    fixture
        .upload_as(
            &fixture.admin,
            &fixture.card,
            &multipart_type,
            multipart("other", "audio/wav", &wav()),
        )
        .await
        .assert_invalid("file", "file_missing");
    fixture
        .upload_as(
            &fixture.admin,
            &fixture.card,
            &multipart_type,
            multipart("file", "audio/wav", b""),
        )
        .await
        .assert_invalid("file", "file_empty");
    assert_eq!(fixture.store.keys(), Vec::<String>::new());
    assert_eq!(audio_rows(&pool).await, 0);
}

#[sqlx::test]
async fn refuses_files_over_10_mb(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let mut file = wav();
    file.resize(10 * 1024 * 1024, 0);
    let at_limit = fixture.upload(&fixture.card, &file).await;
    assert_eq!(at_limit.status, StatusCode::OK, "{}", at_limit.body);

    file.push(0);
    fixture
        .upload(&fixture.card, &file)
        .await
        .assert_problem(StatusCode::PAYLOAD_TOO_LARGE, "too-large");
    file.resize(12 * 1024 * 1024, 0);
    fixture
        .upload(&fixture.card, &file)
        .await
        .assert_problem(StatusCode::PAYLOAD_TOO_LARGE, "too-large");
    assert_eq!(fixture.store.keys().len(), 1);
}

#[sqlx::test]
async fn replacing_or_removing_a_recording_leaves_no_orphan(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;

    let first = fixture.upload(&fixture.card, &wav()).await.body;
    let second = fixture.upload(&fixture.card, &wav()).await.body;
    assert_ne!(first["id"], second["id"]);
    assert_eq!(
        fixture.store.keys(),
        [format!("audio/{}", second["id"].as_str().unwrap())]
    );
    assert_eq!(audio_rows(&pool).await, 1);

    fixture.validate(&fixture.card).await;
    let removed = fixture
        .server
        .send(
            Method::DELETE,
            &format!("{}/audio", fixture.card),
            Some(&fixture.admin),
            None,
        )
        .await;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    let card = fixture.get(&fixture.card).await;
    assert_eq!(card["audio"], Value::Null);
    assert_eq!(card["status"], "to_review");
    assert_eq!(fixture.store.keys(), Vec::<String>::new());
    assert_eq!(audio_rows(&pool).await, 0);
    let again = fixture
        .server
        .send(
            Method::DELETE,
            &format!("{}/audio", fixture.card),
            Some(&fixture.admin),
            None,
        )
        .await;
    assert_eq!(again.status, StatusCode::NO_CONTENT);

    fixture.upload(&fixture.card, &wav()).await;
    fixture.upload(&fixture.lesson, &wav()).await;
    assert_eq!(fixture.store.keys().len(), 2);
    let deleted = fixture
        .server
        .send(Method::DELETE, &fixture.card, Some(&fixture.admin), None)
        .await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    let deleted = fixture
        .server
        .send(Method::DELETE, &fixture.lesson, Some(&fixture.admin), None)
        .await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    assert_eq!(fixture.store.keys(), Vec::<String>::new());
    assert_eq!(audio_rows(&pool).await, 0);
}

#[sqlx::test]
async fn plays_the_whole_file_or_a_range(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let audio = fixture.upload(&fixture.lesson, &wav()).await.body;
    let id = audio["id"].as_str().unwrap();
    let file = wav();

    let whole = fixture.play(Some(&fixture.admin), id, None).await;
    assert_eq!(whole.status, StatusCode::OK);
    assert_eq!(whole.headers[header::CONTENT_TYPE], "audio/wav");
    assert_eq!(whole.headers[header::CONTENT_LENGTH], "8044");
    assert_eq!(whole.headers[header::ACCEPT_RANGES], "bytes");
    assert_eq!(whole.headers[header::CACHE_CONTROL], "private");
    assert_eq!(whole.bytes, file);

    let part = fixture
        .play(Some(&fixture.admin), id, Some("bytes=0-3"))
        .await;
    assert_eq!(part.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(part.headers[header::CONTENT_RANGE], "bytes 0-3/8044");
    assert_eq!(part.headers[header::CONTENT_LENGTH], "4");
    assert_eq!(part.bytes, b"RIFF");

    let tail = fixture
        .play(Some(&fixture.admin), id, Some("bytes=8040-"))
        .await;
    assert_eq!(tail.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(tail.headers[header::CONTENT_RANGE], "bytes 8040-8043/8044");

    let past = fixture
        .play(Some(&fixture.admin), id, Some("bytes=8044-"))
        .await;
    past.assert_problem(StatusCode::RANGE_NOT_SATISFIABLE, "range-not-satisfiable");
    assert_eq!(past.headers[header::CONTENT_RANGE], "bytes */8044");
}

#[sqlx::test]
async fn learners_play_only_recordings_of_visible_content(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let audio = fixture.upload(&fixture.card, &wav()).await.body;
    let id = audio["id"].as_str().unwrap();
    create_account(&pool, "aicha", "user").await;
    let learner = fixture.server.session("aicha").await;

    let anonymous = fixture.play(None, id, None).await;
    anonymous.assert_problem(StatusCode::UNAUTHORIZED, "unauthenticated");
    let hidden = fixture.play(Some(&learner), id, None).await;
    hidden.assert_problem(StatusCode::NOT_FOUND, "not-found");
    let staff = fixture.play(Some(&fixture.admin), id, None).await;
    assert_eq!(staff.status, StatusCode::OK);

    fixture.validate(&fixture.card).await;
    fixture.validate(&fixture.lesson).await;
    let unit_to_review = fixture.play(Some(&learner), id, None).await;
    unit_to_review.assert_problem(StatusCode::NOT_FOUND, "not-found");

    fixture.validate(&fixture.unit).await;
    let shown = fixture.play(Some(&learner), id, None).await;
    assert_eq!(shown.status, StatusCode::OK);

    fixture
        .upload_as(
            &learner,
            &fixture.card,
            &format!("multipart/form-data; boundary={BOUNDARY}"),
            multipart("file", "audio/wav", &wav()),
        )
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");
    let unknown = fixture
        .play(
            Some(&fixture.admin),
            "00000000-0000-0000-0000-000000000000",
            None,
        )
        .await;
    unknown.assert_problem(StatusCode::NOT_FOUND, "not-found");
}

#[sqlx::test]
async fn uploads_to_an_unknown_item_store_nothing(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;

    fixture
        .upload("/api/v1/cards/00000000-0000-0000-0000-000000000000", &wav())
        .await
        .assert_problem(StatusCode::NOT_FOUND, "not-found");
    assert_eq!(fixture.store.keys(), Vec::<String>::new());
}

#[sqlx::test]
async fn stores_the_length_the_client_measured_for_formats_the_server_cannot_read(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let multipart_type = format!("multipart/form-data; boundary={BOUNDARY}");

    let answer = fixture
        .upload_as(
            &fixture.admin,
            &fixture.card,
            &multipart_type,
            multipart_with_duration(&mp4(), "4200"),
        )
        .await;

    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(answer.body["contentType"], "audio/mp4");
    assert_eq!(answer.body["durationMs"], 4_200);
    assert_eq!(fixture.get(&fixture.card).await["audio"], answer.body);

    let mut ahead = duration_part("1500");
    ahead.extend(multipart("file", "audio/mp4", &mp4()));
    let answer = fixture
        .upload_as(&fixture.admin, &fixture.lesson, &multipart_type, ahead)
        .await;
    assert_eq!(answer.body["durationMs"], 1_500, "{}", answer.body);
}

#[sqlx::test]
async fn the_length_the_server_reads_wins_over_the_claimed_one(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;

    let answer = fixture
        .upload_as(
            &fixture.admin,
            &fixture.card,
            &format!("multipart/form-data; boundary={BOUNDARY}"),
            multipart_with_duration(&wav(), "9000"),
        )
        .await;

    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(answer.body["durationMs"], 1_000);
}

#[sqlx::test]
async fn leaves_the_length_empty_when_nobody_knows_it(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;

    let answer = fixture
        .upload_as(
            &fixture.admin,
            &fixture.card,
            &format!("multipart/form-data; boundary={BOUNDARY}"),
            multipart("file", "audio/mp4", &mp4()),
        )
        .await;

    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(answer.body["durationMs"], Value::Null);
}

#[sqlx::test]
async fn refuses_a_claimed_length_that_makes_no_sense(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let multipart_type = format!("multipart/form-data; boundary={BOUNDARY}");

    for value in ["0", "-5", "600001", "4294967296", "abc", "", "1.5", "NaN"] {
        fixture
            .upload_as(
                &fixture.admin,
                &fixture.card,
                &multipart_type,
                multipart_with_duration(&mp4(), value),
            )
            .await
            .assert_invalid("durationMs", "duration_invalid");
    }
    let at_limit = fixture
        .upload_as(
            &fixture.admin,
            &fixture.card,
            &multipart_type,
            multipart_with_duration(&mp4(), "600000"),
        )
        .await;
    assert_eq!(at_limit.body["durationMs"], 600_000, "{}", at_limit.body);
    assert_eq!(fixture.store.keys().len(), 1);
    assert_eq!(audio_rows(&pool).await, 1);
}
