mod common;

use axum::http::{Method, StatusCode, header};
use common::{Answer, Server, create_account};
use serde_json::{Value, json};
use sqlx::PgPool;

impl Server {
    async fn post(&self, cookie: &str, uri: &str, body: &Value) -> Answer {
        self.send(Method::POST, uri, Some(cookie), Some(body)).await
    }

    async fn put(&self, cookie: &str, uri: &str, body: &Value) -> Answer {
        self.send(Method::PUT, uri, Some(cookie), Some(body)).await
    }

    async fn get(&self, cookie: &str, uri: &str) -> Answer {
        self.send(Method::GET, uri, Some(cookie), None).await
    }

    async fn delete(&self, cookie: &str, uri: &str) -> Answer {
        self.send(Method::DELETE, uri, Some(cookie), None).await
    }

    /// Creates an item and returns it, asserting a 201.
    async fn create(&self, cookie: &str, uri: &str, body: &Value) -> Value {
        let answer = self.post(cookie, uri, body).await;
        assert_eq!(answer.status, StatusCode::CREATED, "{}", answer.body);
        let id = answer.body["id"].as_str().unwrap();
        assert_eq!(answer.headers[header::LOCATION], format!("{uri}/{id}"));
        answer.body
    }

    async fn unit(&self, cookie: &str, track: &str, title: &str) -> String {
        let body = json!({ "track": track, "title": title });
        id(&self.create(cookie, "/api/v1/units", &body).await)
    }

    async fn lesson(&self, cookie: &str, unit_id: &str, title: &str) -> String {
        let body = json!({ "unitId": unit_id, "title": title });
        id(&self.create(cookie, "/api/v1/lessons", &body).await)
    }

    async fn card(&self, cookie: &str, lesson_id: &str, arabic: &str) -> String {
        let body = json!({ "lessonId": lesson_id, "arabic": arabic, "variety": "classique" });
        id(&self.create(cookie, "/api/v1/cards", &body).await)
    }

    async fn set_status(&self, cookie: &str, item: &str, status: &str) -> Answer {
        self.put(
            cookie,
            &format!("{item}/status"),
            &json!({ "status": status }),
        )
        .await
    }

    /// The titles of the review list for `query`, in order.
    async fn titles(&self, cookie: &str, query: &str) -> Vec<String> {
        let answer = self
            .get(cookie, &format!("/api/v1/content-items{query}"))
            .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        answer.body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["title"].as_str().unwrap().to_owned())
            .collect()
    }
}

fn id(item: &Value) -> String {
    item["id"].as_str().unwrap().to_owned()
}

/// The positions of the units of `track`, by title, in order.
async fn unit_order(pool: &PgPool, track: &str) -> Vec<(String, i32)> {
    sqlx::query_as(
        "SELECT title, position FROM units WHERE track = $1::content_track ORDER BY position",
    )
    .bind(track)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn lesson_order(pool: &PgPool, unit_id: &str) -> Vec<(String, i32)> {
    sqlx::query_as("SELECT title, position FROM lessons WHERE unit_id = $1::uuid ORDER BY position")
        .bind(unit_id)
        .fetch_all(pool)
        .await
        .unwrap()
}

fn order(items: &[(&str, i32)]) -> Vec<(String, i32)> {
    items
        .iter()
        .map(|(title, position)| ((*title).to_owned(), *position))
        .collect()
}

/// An admin's session, on a server of its own.
async fn admin(pool: &PgPool, username: &str) -> (Server, String) {
    create_account(pool, username, "admin").await;
    let server = Server::new(pool);
    let cookie = server.session(username).await;
    (server, cookie)
}

#[sqlx::test]
async fn content_is_for_admins_only(pool: PgPool) {
    let (server, admin) = admin(&pool, "nour").await;
    let unit = server.unit(&admin, "arabe", "L'alphabet").await;
    create_account(&pool, "aicha", "user").await;
    let user = server.session("aicha").await;

    for (method, uri) in [
        (Method::GET, "/api/v1/content-items".to_owned()),
        (Method::GET, format!("/api/v1/units/{unit}")),
        (Method::DELETE, format!("/api/v1/units/{unit}")),
    ] {
        server
            .send(method.clone(), &uri, Some(&user), None)
            .await
            .assert_problem(StatusCode::FORBIDDEN, "forbidden");
        server
            .send(method, &uri, None, None)
            .await
            .assert_problem(StatusCode::UNAUTHORIZED, "unauthenticated");
    }
    server
        .post(
            &user,
            "/api/v1/units",
            &json!({ "track": "islam", "title": "x" }),
        )
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");
    server
        .set_status(&user, &format!("/api/v1/units/{unit}"), "validated")
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");
}

#[sqlx::test]
async fn creates_units_lessons_and_cards_to_review(pool: PgPool) {
    let (server, admin) = admin(&pool, "nour").await;

    let unit = server
        .create(
            &admin,
            "/api/v1/units",
            &json!({ "track": "arabe", "title": "  L'alphabet " }),
        )
        .await;
    assert_eq!(unit["title"], "L'alphabet");
    assert_eq!(unit["position"], 1);
    assert_eq!(unit["status"], "to_review");
    assert_eq!(unit["createdBy"]["username"], "nour");
    assert_eq!(unit["validatedBy"], Value::Null);

    let lesson = server
        .create(
            &admin,
            "/api/v1/lessons",
            &json!({ "unitId": unit["id"], "title": "Les 4 formes des lettres" }),
        )
        .await;
    assert_eq!(lesson["unit"]["title"], "L'alphabet");
    assert_eq!(lesson["track"], "arabe");

    let card = server
        .create(
            &admin,
            "/api/v1/cards",
            &json!({
                "lessonId": lesson["id"],
                "arabic": "شُكْرًا",
                "transliteration": "shukran",
                "meaningFr": "",
                "variety": "classique",
            }),
        )
        .await;
    assert_eq!(card["arabic"], "شُكْرًا");
    assert_eq!(card["meaningFr"], Value::Null);
    assert_eq!(card["audio"], Value::Null);
    assert_eq!(card["lesson"]["title"], "Les 4 formes des lettres");
    assert_eq!(card["status"], "to_review");

    let fetched = server
        .get(&admin, &format!("/api/v1/cards/{}", id(&card)))
        .await;
    assert_eq!(fetched.status, StatusCode::OK);
    assert_eq!(fetched.body, card);
}

#[sqlx::test]
async fn refuses_invalid_fields_and_missing_parents(pool: PgPool) {
    let (server, admin) = admin(&pool, "nour").await;
    let unit = server.unit(&admin, "islam", "Les piliers").await;
    let lesson = server.lesson(&admin, &unit, "La shahada").await;

    server
        .post(
            &admin,
            "/api/v1/units",
            &json!({ "track": "islam", "title": "  " }),
        )
        .await
        .assert_invalid("title", "title_length");
    server
        .post(
            &admin,
            "/api/v1/lessons",
            &json!({ "unitId": lesson, "title": "Perdue" }),
        )
        .await
        .assert_invalid("unitId", "unit_not_found");
    server
        .post(
            &admin,
            "/api/v1/cards",
            &json!({ "lessonId": unit, "arabic": "سلام", "variety": "darija" }),
        )
        .await
        .assert_invalid("lessonId", "lesson_not_found");
    server
        .post(
            &admin,
            "/api/v1/cards",
            &json!({ "lessonId": lesson, "arabic": "salam", "variety": "darija" }),
        )
        .await
        .assert_invalid("arabic", "arabic_script");
    server
        .post(
            &admin,
            "/api/v1/units",
            &json!({ "track": "islam", "title": "x", "position": 0 }),
        )
        .await
        .assert_invalid("position", "range");
    server
        .get(&admin, "/api/v1/cards/not-a-uuid")
        .await
        .assert_problem(StatusCode::NOT_FOUND, "not-found");
}

#[sqlx::test]
async fn any_admin_validates_and_any_edit_sends_back_to_review(pool: PgPool) {
    let (server, author) = admin(&pool, "nour").await;
    create_account(&pool, "bilal", "admin").await;
    let reviewer = server.session("bilal").await;
    let unit = server.unit(&author, "arabe", "Salutations").await;
    let lesson = server.lesson(&author, &unit, "En darija").await;
    let card = format!(
        "/api/v1/cards/{}",
        server.card(&author, &lesson, "واش راك؟").await
    );

    let validated = server.set_status(&reviewer, &card, "validated").await;
    assert_eq!(validated.status, StatusCode::OK, "{}", validated.body);
    assert_eq!(validated.body["status"], "validated");
    assert_eq!(validated.body["validatedBy"]["username"], "bilal");
    let validated_at = validated.body["validatedAt"].clone();
    assert!(validated_at.is_string());

    let again = server.set_status(&author, &card, "validated").await;
    assert_eq!(again.body["validatedBy"]["username"], "bilal");
    assert_eq!(again.body["validatedAt"], validated_at);

    let same = json!({
        "lessonId": lesson,
        "arabic": "واش راك؟",
        "variety": "classique",
        "position": 1,
    });
    let unchanged = server.put(&author, &card, &same).await;
    assert_eq!(unchanged.status, StatusCode::OK, "{}", unchanged.body);
    assert_eq!(unchanged.body["status"], "validated");

    let edited = server
        .put(
            &author,
            &card,
            &json!({
                "lessonId": lesson,
                "arabic": "واش راك؟",
                "transliteration": "wach rak ?",
                "variety": "darija",
                "position": 1,
            }),
        )
        .await;
    assert_eq!(edited.status, StatusCode::OK, "{}", edited.body);
    assert_eq!(edited.body["status"], "to_review");
    assert_eq!(edited.body["validatedBy"], Value::Null);
    assert_eq!(edited.body["validatedAt"], Value::Null);
    assert_eq!(edited.body["updatedBy"]["username"], "nour");
    assert_eq!(edited.body["variety"], "darija");

    let own = server.set_status(&author, &card, "validated").await;
    assert_eq!(own.body["validatedBy"]["username"], "nour");
    let back = server.set_status(&reviewer, &card, "to_review").await;
    assert_eq!(back.body["status"], "to_review");
    assert_eq!(back.body["validatedBy"], Value::Null);

    let unit_uri = format!("/api/v1/units/{unit}");
    server.set_status(&reviewer, &unit_uri, "validated").await;
    let renamed = server
        .put(
            &reviewer,
            &unit_uri,
            &json!({ "track": "arabe", "title": "Salutations en darija", "position": 1 }),
        )
        .await;
    assert_eq!(renamed.body["status"], "to_review");

    server
        .set_status(
            &reviewer,
            "/api/v1/lessons/00000000-0000-0000-0000-000000000000",
            "validated",
        )
        .await
        .assert_problem(StatusCode::NOT_FOUND, "not-found");
}

#[sqlx::test]
async fn a_move_keeps_the_status_but_an_edit_of_content_resets_it(pool: PgPool) {
    let (server, author) = admin(&pool, "nour").await;
    create_account(&pool, "bilal", "admin").await;
    let reviewer = server.session("bilal").await;
    let alphabet = server.unit(&author, "arabe", "Alphabet").await;
    let greetings = server.unit(&author, "arabe", "Salutations").await;
    let first = server.lesson(&author, &alphabet, "Lettres 1").await;
    let second = server.lesson(&author, &alphabet, "Lettres 2").await;
    let first_card = server.card(&author, &first, "ب").await;
    server.card(&author, &first, "ت").await;
    let second_card = server.card(&author, &second, "ث").await;

    let card_uri = format!("/api/v1/cards/{first_card}");
    let validated = server.set_status(&reviewer, &card_uri, "validated").await;
    let validated_at = validated.body["validatedAt"].clone();
    let card = |lesson: &str, arabic: &str, position: i32| {
        json!({
            "lessonId": lesson,
            "arabic": arabic,
            "variety": "classique",
            "position": position,
        })
    };

    let reordered = server.put(&author, &card_uri, &card(&first, "ب", 2)).await;
    assert_eq!(reordered.status, StatusCode::OK, "{}", reordered.body);
    assert_eq!(reordered.body["position"], 2);
    assert_eq!(reordered.body["status"], "validated");
    assert_eq!(reordered.body["validatedBy"]["username"], "bilal");
    assert_eq!(reordered.body["validatedAt"], validated_at);
    assert_eq!(reordered.body["updatedBy"]["username"], "nour");

    let moved = server.put(&author, &card_uri, &card(&second, "ب", 1)).await;
    assert_eq!(moved.status, StatusCode::OK, "{}", moved.body);
    assert_eq!(moved.body["lesson"]["id"], json!(second));
    assert_eq!(moved.body["status"], "validated");
    assert_eq!(moved.body["validatedAt"], validated_at);

    let sibling = server
        .get(&author, &format!("/api/v1/cards/{second_card}"))
        .await;
    assert_eq!(sibling.body["position"], 2);

    let both = server.put(&author, &card_uri, &card(&first, "بـ", 2)).await;
    assert_eq!(both.status, StatusCode::OK, "{}", both.body);
    assert_eq!(both.body["position"], 2);
    assert_eq!(both.body["status"], "to_review");
    assert_eq!(both.body["validatedBy"], Value::Null);

    server.set_status(&reviewer, &card_uri, "validated").await;
    let edited = server.put(&author, &card_uri, &card(&first, "با", 2)).await;
    assert_eq!(edited.body["status"], "to_review");
    assert_eq!(edited.body["validatedAt"], Value::Null);

    let lesson_uri = format!("/api/v1/lessons/{second}");
    server.set_status(&reviewer, &lesson_uri, "validated").await;
    let lesson_moved = server
        .put(
            &author,
            &lesson_uri,
            &json!({ "unitId": greetings, "title": "Lettres 2", "position": 1 }),
        )
        .await;
    assert_eq!(lesson_moved.status, StatusCode::OK, "{}", lesson_moved.body);
    assert_eq!(lesson_moved.body["unit"]["title"], "Salutations");
    assert_eq!(lesson_moved.body["status"], "validated");
    let lesson_renamed = server
        .put(
            &author,
            &lesson_uri,
            &json!({ "unitId": alphabet, "title": "Lettres 3", "position": 1 }),
        )
        .await;
    assert_eq!(lesson_renamed.body["status"], "to_review");

    let unit_uri = format!("/api/v1/units/{greetings}");
    server.set_status(&reviewer, &unit_uri, "validated").await;
    let unit_moved = server
        .put(
            &author,
            &unit_uri,
            &json!({ "track": "arabe", "title": "Salutations", "position": 1 }),
        )
        .await;
    assert_eq!(unit_moved.body["position"], 1);
    assert_eq!(unit_moved.body["status"], "validated");
    let unit_edited = server
        .put(
            &author,
            &unit_uri,
            &json!({ "track": "arabe", "title": "Politesse", "position": 2 }),
        )
        .await;
    assert_eq!(unit_edited.body["position"], 2);
    assert_eq!(unit_edited.body["status"], "to_review");
}

#[sqlx::test]
async fn keeps_positions_dense_when_inserting_moving_and_deleting(pool: PgPool) {
    let (server, admin) = admin(&pool, "nour").await;
    let alphabet = server.unit(&admin, "arabe", "Alphabet").await;
    server.unit(&admin, "arabe", "Salutations").await;
    server.unit(&admin, "islam", "Piliers").await;
    let first = server
        .create(
            &admin,
            "/api/v1/units",
            &json!({ "track": "arabe", "title": "Sons", "position": 1 }),
        )
        .await;
    assert_eq!(first["position"], 1);
    let last = server
        .create(
            &admin,
            "/api/v1/units",
            &json!({ "track": "arabe", "title": "Chiffres", "position": 99 }),
        )
        .await;
    assert_eq!(last["position"], 4);
    assert_eq!(
        unit_order(&pool, "arabe").await,
        order(&[
            ("Sons", 1),
            ("Alphabet", 2),
            ("Salutations", 3),
            ("Chiffres", 4)
        ])
    );

    let moved = server
        .put(
            &admin,
            &format!("/api/v1/units/{alphabet}"),
            &json!({ "track": "arabe", "title": "Alphabet", "position": 4 }),
        )
        .await;
    assert_eq!(moved.body["position"], 4);
    assert_eq!(
        unit_order(&pool, "arabe").await,
        order(&[
            ("Sons", 1),
            ("Salutations", 2),
            ("Chiffres", 3),
            ("Alphabet", 4)
        ])
    );

    server
        .put(
            &admin,
            &format!("/api/v1/units/{alphabet}"),
            &json!({ "track": "islam", "title": "Alphabet", "position": 1 }),
        )
        .await;
    assert_eq!(
        unit_order(&pool, "arabe").await,
        order(&[("Sons", 1), ("Salutations", 2), ("Chiffres", 3)])
    );
    assert_eq!(
        unit_order(&pool, "islam").await,
        order(&[("Alphabet", 1), ("Piliers", 2)])
    );

    let deleted = server
        .delete(&admin, &format!("/api/v1/units/{}", id(&first)))
        .await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    assert_eq!(
        unit_order(&pool, "arabe").await,
        order(&[("Salutations", 1), ("Chiffres", 2)])
    );
}

#[sqlx::test]
async fn moves_a_lesson_to_another_unit(pool: PgPool) {
    let (server, admin) = admin(&pool, "nour").await;
    let alphabet = server.unit(&admin, "arabe", "Alphabet").await;
    let greetings = server.unit(&admin, "arabe", "Salutations").await;
    server.lesson(&admin, &alphabet, "Lettres 1").await;
    let moving = server.lesson(&admin, &alphabet, "Bonjour").await;
    server.lesson(&admin, &alphabet, "Lettres 2").await;
    server.lesson(&admin, &greetings, "Au revoir").await;

    let moved = server
        .put(
            &admin,
            &format!("/api/v1/lessons/{moving}"),
            &json!({ "unitId": greetings, "title": "Bonjour", "position": 1 }),
        )
        .await;
    assert_eq!(moved.status, StatusCode::OK, "{}", moved.body);
    assert_eq!(moved.body["unit"]["title"], "Salutations");
    assert_eq!(
        lesson_order(&pool, &alphabet).await,
        order(&[("Lettres 1", 1), ("Lettres 2", 2)])
    );
    assert_eq!(
        lesson_order(&pool, &greetings).await,
        order(&[("Bonjour", 1), ("Au revoir", 2)])
    );

    server
        .put(
            &admin,
            &format!("/api/v1/lessons/{moving}"),
            &json!({ "unitId": moving, "title": "Bonjour", "position": 1 }),
        )
        .await
        .assert_invalid("unitId", "unit_not_found");
    assert_eq!(
        lesson_order(&pool, &greetings).await,
        order(&[("Bonjour", 1), ("Au revoir", 2)])
    );
}

#[sqlx::test]
async fn deletes_only_empty_containers_and_again_harmlessly(pool: PgPool) {
    let (server, admin) = admin(&pool, "nour").await;
    let unit = server.unit(&admin, "islam", "La prière").await;
    let lesson = server.lesson(&admin, &unit, "Les étapes d'une rakʿa").await;
    let card = server.card(&admin, &lesson, "ٱللَّهُ أَكْبَرُ").await;

    server
        .delete(&admin, &format!("/api/v1/units/{unit}"))
        .await
        .assert_problem(StatusCode::CONFLICT, "not-empty");
    server
        .delete(&admin, &format!("/api/v1/lessons/{lesson}"))
        .await
        .assert_problem(StatusCode::CONFLICT, "not-empty");

    for uri in [
        format!("/api/v1/cards/{card}"),
        format!("/api/v1/lessons/{lesson}"),
        format!("/api/v1/units/{unit}"),
    ] {
        assert_eq!(
            server.delete(&admin, &uri).await.status,
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            server.delete(&admin, &uri).await.status,
            StatusCode::NO_CONTENT
        );
        server
            .get(&admin, &uri)
            .await
            .assert_problem(StatusCode::NOT_FOUND, "not-found");
    }
}

#[sqlx::test]
async fn lists_content_with_filters_counts_and_pages(pool: PgPool) {
    let (server, admin) = admin(&pool, "nour").await;
    let alphabet = server.unit(&admin, "arabe", "L'alphabet").await;
    let letters = server.lesson(&admin, &alphabet, "Les lettres").await;
    let thanks = server.card(&admin, &letters, "شُكْرًا").await;
    let pillars = server.unit(&admin, "islam", "Les piliers").await;
    server
        .set_status(&admin, &format!("/api/v1/units/{pillars}"), "validated")
        .await;

    assert_eq!(
        server.titles(&admin, "").await,
        ["Les piliers", "شُكْرًا", "Les lettres", "L'alphabet"]
    );

    let all = server.get(&admin, "/api/v1/content-items").await;
    assert_eq!(all.body["total"], 4);
    assert_eq!(
        all.body["statusCounts"],
        json!({ "toReview": 3, "validated": 1 })
    );
    let card = &all.body["items"][1];
    assert_eq!(card["kind"], "card");
    assert_eq!(card["id"], thanks.as_str());
    assert_eq!(card["track"], "arabe");
    assert_eq!(card["parentTitle"], "Les lettres");
    assert_eq!(card["unitId"], alphabet.as_str());
    assert_eq!(card["lessonId"], letters.as_str());
    let lesson = &all.body["items"][2];
    assert_eq!(lesson["unitId"], alphabet.as_str());
    assert_eq!(lesson["lessonId"], Value::Null);
    let unit = &all.body["items"][3];
    assert_eq!(unit["unitId"], Value::Null);
    assert_eq!(unit["lessonId"], Value::Null);

    let to_review = server
        .get(&admin, "/api/v1/content-items?status=to_review&track=arabe")
        .await;
    assert_eq!(to_review.body["total"], 3);
    assert_eq!(
        to_review.body["statusCounts"],
        json!({ "toReview": 3, "validated": 0 })
    );

    assert_eq!(
        server.titles(&admin, "?status=validated").await,
        ["Les piliers"]
    );
    assert_eq!(server.titles(&admin, "?track=islam").await, ["Les piliers"]);
    assert_eq!(server.titles(&admin, "?kind=lesson").await, ["Les lettres"]);
    assert_eq!(
        server.titles(&admin, "?q=LETTRES").await,
        ["شُكْرًا", "Les lettres"]
    );
    assert_eq!(
        server.titles(&admin, "?q=alphabet").await,
        ["Les lettres", "L'alphabet"]
    );
    assert_eq!(
        server.titles(&admin, "?q=%D8%B4%D9%83%D8%B1").await,
        ["شُكْرًا"]
    );

    let page = server
        .get(&admin, "/api/v1/content-items?page=2&perPage=3")
        .await;
    assert_eq!(page.body["page"], 2);
    assert_eq!(page.body["perPage"], 3);
    assert_eq!(page.body["total"], 4);
    assert_eq!(page.body["items"].as_array().unwrap().len(), 1);

    server
        .get(&admin, "/api/v1/content-items?perPage=101")
        .await
        .assert_problem(StatusCode::UNPROCESSABLE_ENTITY, "validation");
    server
        .get(&admin, "/api/v1/content-items?status=draft")
        .await
        .assert_problem(StatusCode::BAD_REQUEST, "malformed-query");
}

#[sqlx::test]
async fn documents_the_content_endpoints(pool: PgPool) {
    let server = Server::new(&pool);
    let document = server.send(Method::GET, "/openapi.json", None, None).await;

    for (path, method) in [
        ("/api/v1/content-items", "get"),
        ("/api/v1/units", "post"),
        ("/api/v1/units/{unitId}", "put"),
        ("/api/v1/lessons/{lessonId}", "delete"),
        ("/api/v1/cards/{cardId}", "get"),
        ("/api/v1/cards/{cardId}/status", "put"),
        ("/api/v1/cards/{cardId}/review-note", "put"),
        ("/api/v1/units/{unitId}/review-note", "put"),
        ("/api/v1/cards/{cardId}/audio", "put"),
        ("/api/v1/lessons/{lessonId}/audio", "delete"),
        ("/api/v1/audio/{audioId}", "get"),
    ] {
        let operation = &document.body["paths"][path][method];
        assert_eq!(operation["tags"], json!(["Content"]), "{method} {path}");
        assert!(
            operation["security"].is_array(),
            "{method} {path} needs a session"
        );
    }
}
