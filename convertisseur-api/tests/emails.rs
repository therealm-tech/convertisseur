mod common;

use axum::http::{Method, StatusCode};
use common::{Answer, PASSWORD, Server, create_account};
use convertisseur_api::ensure_super_admin;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

impl Server {
    async fn set_email(&self, cookie: &str, id: Uuid, email: &str) -> Answer {
        self.send(
            Method::PUT,
            &format!("/api/v1/users/{id}/email"),
            Some(cookie),
            Some(&json!({ "email": email })),
        )
        .await
    }

    async fn current_session(&self, cookie: &str) -> Answer {
        self.send(Method::GET, "/api/v1/sessions/current", Some(cookie), None)
            .await
    }
}

async fn email_of(pool: &PgPool, id: Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn an_account_without_an_email_sets_one_and_logs_in_with_it(pool: PgPool) {
    let server = Server::new(&pool);
    let id = create_account(&pool, "francis", "user").await;
    let session = server.session("francis").await;
    assert_eq!(
        server.current_session(&session).await.body["user"]["email"],
        json!(null)
    );

    let answer = server.set_email(&session, id, "Francis@example.com").await;

    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(answer.body, json!({ "email": "Francis@example.com" }));
    assert_eq!(
        server.current_session(&session).await.body["user"]["email"],
        "Francis@example.com"
    );
    assert_eq!(
        server.login("francis@example.com", PASSWORD).await.status,
        StatusCode::CREATED
    );
}

#[sqlx::test]
async fn an_email_can_be_changed_and_set_again(pool: PgPool) {
    let server = Server::new(&pool);
    let id = create_account(&pool, "francis", "user").await;
    let session = server.session("francis").await;

    for email in ["francis@example.com", "f@example.org", "f@example.org"] {
        let answer = server.set_email(&session, id, email).await;
        assert_eq!(answer.status, StatusCode::OK, "{email}");
    }

    assert_eq!(email_of(&pool, id).await.as_deref(), Some("f@example.org"));
    assert_eq!(
        server.login("francis@example.com", PASSWORD).await.status,
        StatusCode::UNAUTHORIZED,
        "the previous email no longer logs in"
    );
}

#[sqlx::test]
async fn the_super_admin_sets_its_own_email(pool: PgPool) {
    let id = ensure_super_admin(&pool, "chef", PASSWORD.to_owned())
        .await
        .unwrap();
    let server = Server::new(&pool);
    let session = server.session("chef").await;

    let answer = server.set_email(&session, id, "chef@example.com").await;

    assert_eq!(answer.status, StatusCode::OK);
    ensure_super_admin(&pool, "chef", PASSWORD.to_owned())
        .await
        .unwrap();
    assert_eq!(
        email_of(&pool, id).await.as_deref(),
        Some("chef@example.com"),
        "the bootstrap keeps it"
    );
}

#[sqlx::test]
async fn refuses_an_email_another_account_has_in_any_case(pool: PgPool) {
    let server = Server::new(&pool);
    let aicha = create_account(&pool, "aicha", "user").await;
    let francis = create_account(&pool, "francis", "user").await;
    let session = server.session("aicha").await;
    server.set_email(&session, aicha, "aicha@example.com").await;
    let session = server.session("francis").await;

    server
        .set_email(&session, francis, "AICHA@example.com")
        .await
        .assert_problem(StatusCode::CONFLICT, "email-taken");

    assert_eq!(email_of(&pool, francis).await, None);
}

#[sqlx::test]
async fn refuses_an_invalid_email(pool: PgPool) {
    let server = Server::new(&pool);
    let id = create_account(&pool, "francis", "user").await;
    let session = server.session("francis").await;

    for email in ["", "francis", "francis@", "fran cis@example.com"] {
        server
            .set_email(&session, id, email)
            .await
            .assert_invalid("email", "email_invalid");
    }
    assert_eq!(email_of(&pool, id).await, None);
}

#[sqlx::test]
async fn sets_the_email_of_the_session_account_only(pool: PgPool) {
    let server = Server::new(&pool);
    create_account(&pool, "nour", "admin").await;
    let francis = create_account(&pool, "francis", "user").await;
    let session = server.session("nour").await;

    server
        .set_email(&session, francis, "francis@example.com")
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");
    let anonymous = server
        .send(
            Method::PUT,
            &format!("/api/v1/users/{francis}/email"),
            None,
            Some(&json!({ "email": "francis@example.com" })),
        )
        .await;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);

    assert_eq!(email_of(&pool, francis).await, None);
}

#[sqlx::test]
async fn waits_for_a_pending_password_change(pool: PgPool) {
    let server = Server::new(&pool);
    let id = create_account(&pool, "francis", "user").await;
    sqlx::query("UPDATE users SET must_change_password = true WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let session = server.session("francis").await;

    server
        .set_email(&session, id, "francis@example.com")
        .await
        .assert_problem(StatusCode::FORBIDDEN, "password-change-required");
}

#[sqlx::test]
async fn documents_the_endpoint(pool: PgPool) {
    let server = Server::new(&pool);

    let document = server.send(Method::GET, "/openapi.json", None, None).await;

    let operation = &document.body["paths"]["/api/v1/users/{userId}/email"]["put"];
    assert_eq!(operation["tags"], json!(["Users"]));
    for status in ["200", "401", "403", "409", "422"] {
        assert!(operation["responses"][status].is_object(), "{status}");
    }
}
