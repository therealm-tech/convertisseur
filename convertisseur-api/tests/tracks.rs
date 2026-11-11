mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use common::{Answer, Server, create_account};
use serde_json::{Value, json};
use sqlx::PgPool;

const BOUNDARY: &str = "convertisseur-test-boundary";

/// A minimal MP4 audio file, sent as the `file` part of a multipart body.
fn recording() -> Vec<u8> {
    let mut body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; \
         filename=\"enregistrement\"\r\nContent-Type: audio/mp4\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(b"\0\0\0\x18ftypM4A \0\0\0\0M4A mp42");
    body.extend_from_slice(&[0; 64]);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

/// An admin who writes and validates content, and a learner who reads it.
struct Fixture {
    server: Server,
    admin: String,
    learner: String,
}

impl Fixture {
    async fn new(pool: &PgPool) -> Self {
        let server = Server::new(pool);
        create_account(pool, "nour", "admin").await;
        create_account(pool, "aicha", "user").await;
        let admin = server.session("nour").await;
        let learner = server.session("aicha").await;
        Self {
            server,
            admin,
            learner,
        }
    }

    async fn create(&self, collection: &str, body: Value) -> String {
        let answer = self
            .server
            .send(
                Method::POST,
                &format!("/api/v1/{collection}"),
                Some(&self.admin),
                Some(&body),
            )
            .await;
        assert_eq!(answer.status, StatusCode::CREATED, "{}", answer.body);
        answer.body["id"].as_str().unwrap().to_owned()
    }

    async fn unit(&self, track: &str, title: &str) -> String {
        self.create("units", json!({ "track": track, "title": title }))
            .await
    }

    async fn lesson(&self, unit: &str, title: &str) -> String {
        self.create("lessons", json!({ "unitId": unit, "title": title }))
            .await
    }

    async fn card(&self, lesson: &str, arabic: &str) -> String {
        self.create(
            "cards",
            json!({ "lessonId": lesson, "arabic": arabic, "variety": "classique" }),
        )
        .await
    }

    async fn validate(&self, collection: &str, id: &str) {
        let answer = self
            .server
            .send(
                Method::PUT,
                &format!("/api/v1/{collection}/{id}/status"),
                Some(&self.admin),
                Some(&json!({ "status": "validated" })),
            )
            .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    }

    /// Uploads a recording for the item, then validates it again, since a
    /// new recording sends it back to review. Returns the recording's URL.
    async fn record(&self, collection: &str, id: &str) -> String {
        let request = Request::put(format!("/api/v1/{collection}/{id}/audio"))
            .header(header::COOKIE, &self.admin)
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .body(Body::from(recording()))
            .unwrap();
        let answer = self.server.request(request).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        self.validate(collection, id).await;
        format!("/api/v1/audio/{}", answer.body["id"].as_str().unwrap())
    }

    async fn read(&self, uri: &str) -> Answer {
        self.server
            .send(Method::GET, uri, Some(&self.learner), None)
            .await
    }

    async fn tracks(&self) -> Value {
        let answer = self.read("/api/v1/tracks").await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        answer.body
    }
}

/// The titles of a track's units and lessons, as `unit: lesson, lesson`.
fn outline(track: &Value) -> Vec<String> {
    track["units"]
        .as_array()
        .unwrap()
        .iter()
        .map(|unit| {
            let lessons: Vec<&str> = unit["lessons"]
                .as_array()
                .unwrap()
                .iter()
                .map(|lesson| lesson["title"].as_str().unwrap())
                .collect();
            format!(
                "{}: {}",
                unit["title"].as_str().unwrap(),
                lessons.join(", ")
            )
        })
        .collect()
}

#[sqlx::test]
async fn lists_both_tracks_empty_until_content_is_validated(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let empty = json!({ "items": [
        { "id": "arabe", "units": [] },
        { "id": "islam", "units": [] },
    ] });
    assert_eq!(fixture.tracks().await, empty);

    let unit = fixture.unit("arabe", "L'alphabet").await;
    let lesson = fixture.lesson(&unit, "Les lettres").await;
    fixture.card(&lesson, "بَ").await;

    assert_eq!(fixture.tracks().await, empty);
}

#[sqlx::test]
async fn shows_only_what_is_validated_with_every_parent(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let alphabet = fixture.unit("arabe", "L'alphabet").await;
    let letters = fixture.lesson(&alphabet, "Les lettres").await;
    let shapes = fixture.lesson(&alphabet, "Les formes").await;
    let ba = fixture.card(&letters, "بَ").await;
    fixture.card(&letters, "تَ").await;
    let greetings = fixture.unit("arabe", "Salutations").await;
    let darija = fixture.lesson(&greetings, "En darija").await;
    let prayer = fixture.unit("islam", "La prière").await;
    for (collection, id) in [
        ("units", &alphabet),
        ("lessons", &letters),
        ("cards", &ba),
        ("lessons", &darija),
        ("units", &prayer),
    ] {
        fixture.validate(collection, id).await;
    }

    let tracks = fixture.tracks().await;

    assert_eq!(outline(&tracks["items"][0]), ["L'alphabet: Les lettres"]);
    assert_eq!(tracks["items"][0]["units"][0]["lessons"][0]["cardCount"], 1);
    assert_eq!(outline(&tracks["items"][1]), ["La prière: "]);
    let hidden_lesson = fixture
        .read(&format!("/api/v1/tracks/arabe/lessons/{shapes}"))
        .await;
    hidden_lesson.assert_problem(StatusCode::NOT_FOUND, "not-found");
    let in_hidden_unit = fixture
        .read(&format!("/api/v1/tracks/arabe/lessons/{darija}"))
        .await;
    in_hidden_unit.assert_problem(StatusCode::NOT_FOUND, "not-found");
    let lesson = fixture
        .read(&format!("/api/v1/tracks/arabe/lessons/{letters}"))
        .await;
    assert_eq!(lesson.status, StatusCode::OK, "{}", lesson.body);
    assert_eq!(lesson.body["cards"].as_array().unwrap().len(), 1);
    assert_eq!(lesson.body["cards"][0]["id"], ba.as_str());

    fixture
        .server
        .send(
            Method::PUT,
            &format!("/api/v1/units/{alphabet}/status"),
            Some(&fixture.admin),
            Some(&json!({ "status": "to_review" })),
        )
        .await;
    let tracks = fixture.tracks().await;
    assert_eq!(outline(&tracks["items"][0]), Vec::<String>::new());
    let unit_back_to_review = fixture
        .read(&format!("/api/v1/tracks/arabe/lessons/{letters}"))
        .await;
    unit_back_to_review.assert_problem(StatusCode::NOT_FOUND, "not-found");
}

#[sqlx::test]
async fn keeps_the_order_of_units_lessons_and_cards(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let second = fixture.unit("arabe", "Deuxième").await;
    let first = fixture
        .create(
            "units",
            json!({ "track": "arabe", "title": "Première", "position": 1 }),
        )
        .await;
    let later = fixture.lesson(&first, "Plus tard").await;
    let sooner = fixture
        .create(
            "lessons",
            json!({ "unitId": first, "title": "D'abord", "position": 1 }),
        )
        .await;
    let ta = fixture.card(&sooner, "تَ").await;
    let ba = fixture
        .create(
            "cards",
            json!({ "lessonId": sooner, "arabic": "بَ", "variety": "classique", "position": 1 }),
        )
        .await;
    for (collection, id) in [
        ("units", &first),
        ("units", &second),
        ("lessons", &later),
        ("lessons", &sooner),
        ("cards", &ta),
        ("cards", &ba),
    ] {
        fixture.validate(collection, id).await;
    }

    let tracks = fixture.tracks().await;
    let lesson = fixture
        .read(&format!("/api/v1/tracks/arabe/lessons/{sooner}"))
        .await;

    assert_eq!(
        outline(&tracks["items"][0]),
        ["Première: D'abord, Plus tard", "Deuxième: "]
    );
    let cards: Vec<&Value> = lesson.body["cards"]
        .as_array()
        .unwrap()
        .iter()
        .map(|card| &card["arabic"])
        .collect();
    assert_eq!(cards, [&json!("بَ"), &json!("تَ")]);
}

#[sqlx::test]
async fn reads_one_track(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let arabe = fixture.unit("arabe", "L'alphabet").await;
    let islam = fixture.unit("islam", "La prière").await;
    fixture.validate("units", &arabe).await;
    fixture.validate("units", &islam).await;

    let track = fixture.read("/api/v1/tracks/islam").await;

    assert_eq!(track.status, StatusCode::OK, "{}", track.body);
    assert_eq!(track.body["id"], "islam");
    assert_eq!(outline(&track.body), ["La prière: "]);
    fixture
        .read("/api/v1/tracks/histoire")
        .await
        .assert_problem(StatusCode::NOT_FOUND, "not-found");
}

#[sqlx::test]
async fn gives_a_lesson_with_its_cards_and_recordings(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let unit = fixture.unit("arabe", "Salutations").await;
    let lesson = fixture.lesson(&unit, "En darija").await;
    let silent = fixture
        .create(
            "cards",
            json!({
                "lessonId": lesson,
                "arabic": "وَاشْ رَاكْ؟",
                "transliteration": "wach rak ?",
                "meaningFr": "ça va ?",
                "variety": "darija",
            }),
        )
        .await;
    let spoken = fixture.card(&lesson, "صَحَّة").await;
    for (collection, id) in [
        ("units", &unit),
        ("lessons", &lesson),
        ("cards", &silent),
        ("cards", &spoken),
    ] {
        fixture.validate(collection, id).await;
    }
    let lesson_audio = fixture.record("lessons", &lesson).await;
    let card_audio = fixture.record("cards", &spoken).await;

    let answer = fixture
        .read(&format!("/api/v1/tracks/arabe/lessons/{lesson}"))
        .await;

    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(
        answer.body,
        json!({
            "id": lesson,
            "track": "arabe",
            "unit": { "id": unit, "title": "Salutations" },
            "title": "En darija",
            "audio": { "url": lesson_audio, "durationMs": null },
            "cards": [
                {
                    "id": silent,
                    "arabic": "وَاشْ رَاكْ؟",
                    "transliteration": "wach rak ?",
                    "meaningFr": "ça va ?",
                    "variety": "darija",
                    "forms": null,
                    "audio": null,
                },
                {
                    "id": spoken,
                    "arabic": "صَحَّة",
                    "transliteration": null,
                    "meaningFr": null,
                    "variety": "classique",
                    "forms": null,
                    "audio": { "url": card_audio, "durationMs": null },
                },
            ],
        })
    );
    for url in [&lesson_audio, &card_audio] {
        assert_eq!(fixture.read(url).await.status, StatusCode::OK, "{url}");
    }
    fixture
        .read(&format!("/api/v1/tracks/islam/lessons/{lesson}"))
        .await
        .assert_problem(StatusCode::NOT_FOUND, "not-found");
}

#[sqlx::test]
async fn refuses_the_recordings_of_content_learners_do_not_see(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let unit = fixture.unit("arabe", "Salutations").await;
    let lesson = fixture.lesson(&unit, "En darija").await;
    let card = fixture.card(&lesson, "صَحَّة").await;
    fixture.validate("lessons", &lesson).await;
    fixture.validate("cards", &card).await;
    let lesson_audio = fixture.record("lessons", &lesson).await;
    let card_audio = fixture.record("cards", &card).await;

    for url in [&lesson_audio, &card_audio] {
        fixture
            .read(url)
            .await
            .assert_problem(StatusCode::NOT_FOUND, "not-found");
        let staff = fixture
            .server
            .send(Method::GET, url, Some(&fixture.admin), None)
            .await;
        assert_eq!(staff.status, StatusCode::OK, "{url}");
    }

    fixture.validate("units", &unit).await;
    for url in [&lesson_audio, &card_audio] {
        assert_eq!(fixture.read(url).await.status, StatusCode::OK, "{url}");
    }
}

#[sqlx::test]
async fn needs_a_session(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let unit = fixture.unit("arabe", "Salutations").await;
    let lesson = fixture.lesson(&unit, "En darija").await;
    fixture.validate("units", &unit).await;
    fixture.validate("lessons", &lesson).await;

    for uri in [
        "/api/v1/tracks".to_owned(),
        "/api/v1/tracks/arabe".to_owned(),
        format!("/api/v1/tracks/arabe/lessons/{lesson}"),
    ] {
        fixture
            .server
            .send(Method::GET, &uri, None, None)
            .await
            .assert_problem(StatusCode::UNAUTHORIZED, "unauthenticated");
    }
}

#[sqlx::test]
async fn waits_for_a_reset_password_to_be_changed(pool: PgPool) {
    let fixture = Fixture::new(&pool).await;
    let unit = fixture.unit("arabe", "Salutations").await;
    let lesson = fixture.lesson(&unit, "En darija").await;
    fixture.validate("units", &unit).await;
    fixture.validate("lessons", &lesson).await;
    sqlx::query("UPDATE users SET must_change_password = TRUE WHERE username = 'aicha'")
        .execute(&pool)
        .await
        .unwrap();

    for uri in [
        "/api/v1/tracks".to_owned(),
        "/api/v1/tracks/arabe".to_owned(),
        format!("/api/v1/tracks/arabe/lessons/{lesson}"),
    ] {
        fixture
            .read(&uri)
            .await
            .assert_problem(StatusCode::FORBIDDEN, "password-change-required");
    }
}

#[sqlx::test]
async fn documents_the_endpoints(pool: PgPool) {
    let server = Server::new(&pool);

    let document = server.send(Method::GET, "/openapi.json", None, None).await;

    let paths = &document.body["paths"];
    for path in [
        "/api/v1/tracks",
        "/api/v1/tracks/{track}",
        "/api/v1/tracks/{track}/lessons/{lessonId}",
    ] {
        let get = &paths[path]["get"];
        assert_eq!(get["tags"], json!(["Tracks"]), "{path}");
        assert_eq!(get["security"], json!([{ "SessionCookie": [] }]), "{path}");
        assert!(get["responses"]["401"].is_object(), "{path}");
        assert!(get["responses"]["403"].is_object(), "{path}");
    }
    let tags: Vec<&Value> = document.body["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tag| &tag["name"])
        .collect();
    assert_eq!(
        tags,
        [
            &json!("Sessions"),
            &json!("Users"),
            &json!("Tracks"),
            &json!("Content")
        ]
    );
}
