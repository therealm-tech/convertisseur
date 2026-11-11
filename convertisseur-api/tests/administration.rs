use std::{net::SocketAddr, sync::OnceLock};

use argon2::{Argon2, PasswordHasher};
use axum::{
    Router,
    body::Body,
    extract::connect_info::MockConnectInfo,
    http::{HeaderMap, Method, Request, StatusCode, header},
};
use convertisseur_api::{
    AppState, ClientSettings, app, ensure_super_admin,
    storage::{MemoryStore, Storage},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const PASSWORD: &str = "a long enough password";
const NEW_PASSWORD: &str = "another long enough password";
const PEER: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 40000);

struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: Value,
}

impl Answer {
    fn problem_type(&self) -> &str {
        assert_eq!(
            self.headers.get(header::CONTENT_TYPE).unwrap(),
            "application/problem+json"
        );
        self.body["type"].as_str().unwrap()
    }

    fn assert_problem(&self, status: StatusCode, kind: &str) {
        assert_eq!(self.status, status, "{}", self.body);
        assert_eq!(
            self.problem_type(),
            format!("urn:convertisseur:problem:{kind}")
        );
    }

    fn cookie(&self) -> String {
        let set_cookie = self.headers[header::SET_COOKIE].to_str().unwrap();
        set_cookie.split(';').next().unwrap().to_owned()
    }
}

/// The app, with the login rate limit of a single client: a test logs in at
/// most 10 times per `Server`.
struct Server {
    router: Router,
}

impl Server {
    fn new(pool: &PgPool) -> Self {
        let state = AppState::new(
            pool.clone(),
            Storage::Memory(MemoryStore::default()),
            ClientSettings::default(),
        );
        let router = app(state, &std::env::temp_dir()).layer(MockConnectInfo(PEER));
        Self { router }
    }

    async fn send(
        &self,
        method: Method,
        uri: &str,
        cookie: Option<&str>,
        body: Option<&Value>,
    ) -> Answer {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        let request = match body {
            Some(body) => request
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string())),
            None => request.body(Body::empty()),
        };
        let response = self.router.clone().oneshot(request.unwrap()).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        Answer {
            status,
            headers,
            body,
        }
    }

    async fn login(&self, username: &str, password: &str) -> Answer {
        let body = json!({ "username": username, "password": password });
        self.send(Method::POST, "/api/v1/sessions", None, Some(&body))
            .await
    }

    /// Logs in as `username`, with [`PASSWORD`]; returns the session cookie.
    async fn session(&self, username: &str) -> String {
        let answer = self.login(username, PASSWORD).await;
        assert_eq!(answer.status, StatusCode::CREATED, "{username}");
        answer.cookie()
    }

    async fn current_session(&self, cookie: &str) -> Answer {
        self.send(Method::GET, "/api/v1/sessions/current", Some(cookie), None)
            .await
    }

    async fn list(&self, cookie: &str, query: &str) -> Answer {
        self.send(
            Method::GET,
            &format!("/api/v1/users{query}"),
            Some(cookie),
            None,
        )
        .await
    }

    async fn reset_password(&self, cookie: &str, id: Uuid) -> Answer {
        self.send(
            Method::POST,
            &format!("/api/v1/users/{id}/password-resets"),
            Some(cookie),
            None,
        )
        .await
    }

    async fn delete(&self, cookie: &str, id: Uuid) -> Answer {
        self.send(
            Method::DELETE,
            &format!("/api/v1/users/{id}"),
            Some(cookie),
            None,
        )
        .await
    }

    async fn set_role(&self, cookie: &str, id: Uuid, role: &str) -> Answer {
        self.send(
            Method::PUT,
            &format!("/api/v1/users/{id}/role"),
            Some(cookie),
            Some(&json!({ "role": role })),
        )
        .await
    }

    async fn change_password(&self, cookie: &str, id: Uuid, current: &str, new: &str) -> Answer {
        self.send(
            Method::PUT,
            &format!("/api/v1/users/{id}/password"),
            Some(cookie),
            Some(&json!({ "currentPassword": current, "newPassword": new })),
        )
        .await
    }
}

/// Hashing is slow in a debug build: every account a test creates shares
/// this hash of [`PASSWORD`].
fn password_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| {
        Argon2::default()
            .hash_password(PASSWORD.as_bytes())
            .unwrap()
            .to_string()
    })
}

async fn create_account(pool: &PgPool, username: &str, role: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO users (username, password_hash, role) VALUES ($1, $2, $3::user_role) \
         RETURNING id",
    )
    .bind(username)
    .bind(password_hash())
    .bind(role)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn super_admin(pool: &PgPool) -> Uuid {
    ensure_super_admin(pool, "chef", PASSWORD.to_owned())
        .await
        .unwrap()
}

async fn role_of(pool: &PgPool, id: Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT role::text FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .unwrap()
}

async fn hash_of(pool: &PgPool, id: Uuid) -> String {
    sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn sessions_of(pool: &PgPool, id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM sessions WHERE user_id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn the_admin_area_needs_an_admin(pool: PgPool) {
    let server = Server::new(&pool);
    create_account(&pool, "aicha", "user").await;
    let other = create_account(&pool, "bilal", "user").await;
    let user = server.session("aicha").await;

    server
        .list(&user, "")
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");
    server
        .reset_password(&user, other)
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");
    server
        .delete(&user, other)
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");
    server
        .set_role(&user, other, "admin")
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");
    assert_eq!(role_of(&pool, other).await.as_deref(), Some("user"));
    assert_eq!(hash_of(&pool, other).await, password_hash());

    let anonymous = server.send(Method::GET, "/api/v1/users", None, None).await;
    anonymous.assert_problem(StatusCode::UNAUTHORIZED, "unauthenticated");
    let anonymous = server
        .send(
            Method::DELETE,
            &format!("/api/v1/users/{other}"),
            None,
            None,
        )
        .await;
    anonymous.assert_problem(StatusCode::UNAUTHORIZED, "unauthenticated");
}

#[sqlx::test]
async fn admins_and_the_super_admin_reset_and_delete_the_roles_below_theirs(pool: PgPool) {
    let server = Server::new(&pool);
    super_admin(&pool).await;
    create_account(&pool, "nour", "admin").await;
    let chef = server.session("chef").await;
    let nour = server.session("nour").await;

    let cases = [
        (&nour, "user", true),
        (&nour, "admin", false),
        (&chef, "user", true),
        (&chef, "admin", true),
    ];
    for (n, (actor, target_role, allowed)) in cases.into_iter().enumerate() {
        let target = create_account(&pool, &format!("reset{n}"), target_role).await;
        let answer = server.reset_password(actor, target).await;
        if allowed {
            assert_eq!(answer.status, StatusCode::CREATED, "reset {n}");
            assert_ne!(hash_of(&pool, target).await, password_hash(), "reset {n}");
        } else {
            answer.assert_problem(StatusCode::FORBIDDEN, "forbidden");
            assert_eq!(hash_of(&pool, target).await, password_hash(), "reset {n}");
        }

        let target = create_account(&pool, &format!("delete{n}"), target_role).await;
        let answer = server.delete(actor, target).await;
        if allowed {
            assert_eq!(answer.status, StatusCode::NO_CONTENT, "delete {n}");
            assert_eq!(role_of(&pool, target).await, None, "delete {n}");
        } else {
            answer.assert_problem(StatusCode::FORBIDDEN, "forbidden");
            assert!(role_of(&pool, target).await.is_some(), "delete {n}");
        }
    }
}

#[sqlx::test]
async fn only_the_super_admin_changes_roles(pool: PgPool) {
    let server = Server::new(&pool);
    super_admin(&pool).await;
    create_account(&pool, "nour", "admin").await;
    let user = create_account(&pool, "aicha", "user").await;
    let admin = create_account(&pool, "bilal", "admin").await;
    let chef = server.session("chef").await;
    let nour = server.session("nour").await;

    server
        .set_role(&nour, user, "admin")
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");
    server
        .set_role(&nour, admin, "user")
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");
    assert_eq!(role_of(&pool, user).await.as_deref(), Some("user"));
    assert_eq!(role_of(&pool, admin).await.as_deref(), Some("admin"));

    let promoted = server.set_role(&chef, user, "admin").await;
    assert_eq!(promoted.status, StatusCode::OK);
    assert_eq!(promoted.body, json!({ "role": "admin" }));
    let demoted = server.set_role(&chef, admin, "user").await;
    assert_eq!(demoted.status, StatusCode::OK);
    assert_eq!(role_of(&pool, user).await.as_deref(), Some("admin"));
    assert_eq!(role_of(&pool, admin).await.as_deref(), Some("user"));

    let unchanged = server.set_role(&chef, user, "admin").await;
    assert_eq!(
        unchanged.status,
        StatusCode::OK,
        "setting the same role again"
    );

    let answer = server.set_role(&chef, user, "super_admin").await;
    answer.assert_problem(StatusCode::UNPROCESSABLE_ENTITY, "validation");
    assert_eq!(
        answer.body["errors"],
        json!([{ "field": "role", "code": "role_not_assignable" }])
    );
    server
        .set_role(&chef, Uuid::nil(), "admin")
        .await
        .assert_problem(StatusCode::NOT_FOUND, "not-found");
}

#[sqlx::test]
async fn a_new_role_applies_to_open_sessions(pool: PgPool) {
    let server = Server::new(&pool);
    super_admin(&pool).await;
    let id = create_account(&pool, "nour", "admin").await;
    let chef = server.session("chef").await;
    let nour = server.session("nour").await;
    assert_eq!(server.list(&nour, "").await.status, StatusCode::OK);

    server.set_role(&chef, id, "user").await;

    server
        .list(&nour, "")
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");
}

#[sqlx::test]
async fn nobody_modifies_the_super_admin(pool: PgPool) {
    let server = Server::new(&pool);
    let chef_id = super_admin(&pool).await;
    create_account(&pool, "nour", "admin").await;
    let chef = server.session("chef").await;
    let nour = server.session("nour").await;
    let hash = hash_of(&pool, chef_id).await;

    for actor in [&chef, &nour] {
        server
            .reset_password(actor, chef_id)
            .await
            .assert_problem(StatusCode::FORBIDDEN, "forbidden");
        server
            .delete(actor, chef_id)
            .await
            .assert_problem(StatusCode::FORBIDDEN, "forbidden");
        server
            .set_role(actor, chef_id, "admin")
            .await
            .assert_problem(StatusCode::FORBIDDEN, "forbidden");
    }
    assert_eq!(
        role_of(&pool, chef_id).await.as_deref(),
        Some("super_admin")
    );
    assert_eq!(hash_of(&pool, chef_id).await, hash);
    assert_eq!(server.current_session(&chef).await.status, StatusCode::OK);
}

#[sqlx::test]
async fn nobody_acts_on_their_own_account_from_the_admin_area(pool: PgPool) {
    let server = Server::new(&pool);
    let id = create_account(&pool, "nour", "admin").await;
    let nour = server.session("nour").await;

    server
        .reset_password(&nour, id)
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");
    server
        .delete(&nour, id)
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");
    server
        .set_role(&nour, id, "user")
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");
    assert_eq!(role_of(&pool, id).await.as_deref(), Some("admin"));
    assert_eq!(server.current_session(&nour).await.status, StatusCode::OK);
}

#[sqlx::test]
async fn deleting_an_account_closes_its_sessions_and_can_be_retried(pool: PgPool) {
    let server = Server::new(&pool);
    create_account(&pool, "nour", "admin").await;
    let id = create_account(&pool, "aicha", "user").await;
    let nour = server.session("nour").await;
    let aicha = server.session("aicha").await;

    assert_eq!(
        server.delete(&nour, id).await.status,
        StatusCode::NO_CONTENT
    );

    server
        .current_session(&aicha)
        .await
        .assert_problem(StatusCode::UNAUTHORIZED, "unauthenticated");
    assert_eq!(sessions_of(&pool, id).await, 0);
    assert_eq!(
        server.delete(&nour, id).await.status,
        StatusCode::NO_CONTENT
    );
    let malformed = server
        .send(Method::DELETE, "/api/v1/users/not-an-id", Some(&nour), None)
        .await;
    malformed.assert_problem(StatusCode::NOT_FOUND, "not-found");
}

#[sqlx::test]
async fn a_reset_password_works_until_it_is_replaced_and_blocks_everything_else(pool: PgPool) {
    let server = Server::new(&pool);
    super_admin(&pool).await;
    let id = create_account(&pool, "nour", "admin").await;
    let chef = server.session("chef").await;
    let old_session = server.session("nour").await;

    let reset = server.reset_password(&chef, id).await;
    assert_eq!(reset.status, StatusCode::CREATED);
    assert_eq!(reset.headers[header::CACHE_CONTROL], "no-store");
    let temporary = reset.body["temporaryPassword"].as_str().unwrap().to_owned();
    assert!(temporary.chars().count() >= 16, "{temporary}");
    let flagged: bool = sqlx::query_scalar("SELECT must_change_password FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(flagged);
    assert_eq!(sessions_of(&pool, id).await, 0);
    server
        .current_session(&old_session)
        .await
        .assert_problem(StatusCode::UNAUTHORIZED, "unauthenticated");
    server
        .login("nour", PASSWORD)
        .await
        .assert_problem(StatusCode::UNAUTHORIZED, "invalid-credentials");

    let login = server.login("nour", &temporary).await;
    assert_eq!(login.status, StatusCode::CREATED);
    assert_eq!(login.body["mustChangePassword"], true);
    let session = login.cookie();
    let current = server.current_session(&session).await;
    assert_eq!(current.status, StatusCode::OK);
    assert_eq!(current.body["mustChangePassword"], true);
    server
        .list(&session, "")
        .await
        .assert_problem(StatusCode::FORBIDDEN, "password-change-required");
    let other = create_account(&pool, "aicha", "user").await;
    server
        .delete(&session, other)
        .await
        .assert_problem(StatusCode::FORBIDDEN, "password-change-required");
    server
        .reset_password(&session, other)
        .await
        .assert_problem(StatusCode::FORBIDDEN, "password-change-required");

    let changed = server
        .change_password(&session, id, &temporary, NEW_PASSWORD)
        .await;
    assert_eq!(changed.status, StatusCode::NO_CONTENT);

    let current = server.current_session(&session).await;
    assert_eq!(current.body["mustChangePassword"], false);
    assert_eq!(server.list(&session, "").await.status, StatusCode::OK);
    server
        .login("nour", &temporary)
        .await
        .assert_problem(StatusCode::UNAUTHORIZED, "invalid-credentials");
    assert_eq!(
        server.login("nour", NEW_PASSWORD).await.status,
        StatusCode::CREATED
    );
}

#[sqlx::test]
async fn logging_out_still_works_before_the_forced_change(pool: PgPool) {
    let server = Server::new(&pool);
    create_account(&pool, "nour", "admin").await;
    let id = create_account(&pool, "aicha", "user").await;
    let nour = server.session("nour").await;
    let temporary = server.reset_password(&nour, id).await.body["temporaryPassword"]
        .as_str()
        .unwrap()
        .to_owned();
    let session = server.login("aicha", &temporary).await.cookie();

    let logout = server
        .send(
            Method::DELETE,
            "/api/v1/sessions/current",
            Some(&session),
            None,
        )
        .await;

    assert_eq!(logout.status, StatusCode::NO_CONTENT);
    assert_eq!(sessions_of(&pool, id).await, 0);
}

#[sqlx::test]
async fn resetting_an_unknown_account_answers_404(pool: PgPool) {
    let server = Server::new(&pool);
    create_account(&pool, "nour", "admin").await;
    let nour = server.session("nour").await;

    server
        .reset_password(&nour, Uuid::nil())
        .await
        .assert_problem(StatusCode::NOT_FOUND, "not-found");
}

#[sqlx::test]
async fn changing_your_password_closes_your_other_sessions(pool: PgPool) {
    let server = Server::new(&pool);
    let id = create_account(&pool, "aicha", "user").await;
    let kept = server.session("aicha").await;
    let phone = server.session("aicha").await;
    let tablet = server.session("aicha").await;
    let bystander_id = create_account(&pool, "bilal", "user").await;
    let bystander = server.session("bilal").await;

    let answer = server
        .change_password(&kept, id, PASSWORD, NEW_PASSWORD)
        .await;

    assert_eq!(answer.status, StatusCode::NO_CONTENT);
    assert_eq!(server.current_session(&kept).await.status, StatusCode::OK);
    for closed in [&phone, &tablet] {
        server
            .current_session(closed)
            .await
            .assert_problem(StatusCode::UNAUTHORIZED, "unauthenticated");
    }
    assert_eq!(sessions_of(&pool, id).await, 1);
    assert_eq!(
        server.current_session(&bystander).await.status,
        StatusCode::OK
    );
    assert_eq!(sessions_of(&pool, bystander_id).await, 1);
    server
        .login("aicha", PASSWORD)
        .await
        .assert_problem(StatusCode::UNAUTHORIZED, "invalid-credentials");
    assert_eq!(
        server.login("aicha", NEW_PASSWORD).await.status,
        StatusCode::CREATED
    );
}

#[sqlx::test]
async fn changing_your_password_checks_both_passwords(pool: PgPool) {
    let server = Server::new(&pool);
    let id = create_account(&pool, "aicha", "user").await;
    let other = create_account(&pool, "bilal", "user").await;
    let session = server.session("aicha").await;
    let phone = server.session("aicha").await;

    let cases = [
        (
            "not the password at all",
            NEW_PASSWORD,
            "currentPassword",
            "password_incorrect",
        ),
        (PASSWORD, "too short", "newPassword", "password_length"),
        (PASSWORD, PASSWORD, "newPassword", "password_unchanged"),
    ];
    for (current, new, field, code) in cases {
        let answer = server.change_password(&session, id, current, new).await;
        answer.assert_problem(StatusCode::UNPROCESSABLE_ENTITY, "validation");
        assert_eq!(
            answer.body["errors"],
            json!([{ "field": field, "code": code }])
        );
    }
    server
        .change_password(&session, other, PASSWORD, NEW_PASSWORD)
        .await
        .assert_problem(StatusCode::FORBIDDEN, "forbidden");

    assert_eq!(hash_of(&pool, id).await, password_hash());
    assert_eq!(hash_of(&pool, other).await, password_hash());
    assert_eq!(server.current_session(&phone).await.status, StatusCode::OK);
}

fn usernames(answer: &Answer) -> Vec<&str> {
    answer.body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["username"].as_str().unwrap())
        .collect()
}

#[sqlx::test]
async fn lists_accounts_by_role_then_username(pool: PgPool) {
    let server = Server::new(&pool);
    super_admin(&pool).await;
    for (username, role) in [
        ("sami", "user"),
        ("nour", "admin"),
        ("AICHA", "user"),
        ("karim_92", "user"),
        ("Bilal", "admin"),
        ("aichatou", "user"),
    ] {
        create_account(&pool, username, role).await;
    }
    let nour = server.session("nour").await;

    let all = server.list(&nour, "").await;
    assert_eq!(all.status, StatusCode::OK);
    assert_eq!(
        usernames(&all),
        [
            "chef", "Bilal", "nour", "AICHA", "aichatou", "karim_92", "sami"
        ]
    );
    assert_eq!(all.body["page"], 1);
    assert_eq!(all.body["perPage"], 50);
    assert_eq!(all.body["total"], 7);
    let nour_item = &all.body["items"][2];
    assert_eq!(nour_item["role"], "admin");
    assert!(nour_item["id"].is_string());
    assert!(nour_item["createdAt"].is_string());
    assert!(nour_item["lastActiveAt"].is_string(), "nour is logged in");
    assert!(
        all.body["items"][1]["lastActiveAt"].is_null(),
        "Bilal never logged in"
    );

    let second = server.list(&nour, "?page=2&perPage=3").await;
    assert_eq!(usernames(&second), ["AICHA", "aichatou", "karim_92"]);
    assert_eq!(second.body["total"], 7);
    let past_the_end = server.list(&nour, "?page=4&perPage=3").await;
    assert_eq!(past_the_end.status, StatusCode::OK);
    assert_eq!(usernames(&past_the_end), Vec::<&str>::new());
    assert_eq!(past_the_end.body["total"], 7);
}

#[sqlx::test]
async fn searches_usernames_and_filters_roles(pool: PgPool) {
    let server = Server::new(&pool);
    super_admin(&pool).await;
    for (username, role) in [
        ("nour", "admin"),
        ("AICHA", "user"),
        ("aichatou", "admin"),
        ("karim_92", "user"),
        ("karim92", "user"),
    ] {
        create_account(&pool, username, role).await;
    }
    let nour = server.session("nour").await;

    let cases = [
        ("?q=aIc", vec!["aichatou", "AICHA"]),
        ("?q=%20aic%20", vec!["aichatou", "AICHA"]),
        ("?q=_", vec!["karim_92"]),
        ("?q=%25", vec![]),
        ("?role=admin", vec!["aichatou", "nour"]),
        ("?role=super_admin", vec!["chef"]),
        ("?q=aic&role=user", vec!["AICHA"]),
        (
            "?q=",
            vec!["chef", "aichatou", "nour", "AICHA", "karim92", "karim_92"],
        ),
    ];
    for (query, expected) in cases {
        let answer = server.list(&nour, query).await;
        assert_eq!(answer.status, StatusCode::OK, "{query}");
        assert_eq!(usernames(&answer), expected, "{query}");
        assert_eq!(answer.body["total"], expected.len(), "{query}");
    }
}

#[sqlx::test]
async fn refuses_a_page_out_of_bounds_or_an_unknown_role(pool: PgPool) {
    let server = Server::new(&pool);
    create_account(&pool, "nour", "admin").await;
    let nour = server.session("nour").await;

    for (query, field) in [
        ("?perPage=101", "perPage"),
        ("?perPage=0", "perPage"),
        ("?page=0", "page"),
    ] {
        let answer = server.list(&nour, query).await;
        answer.assert_problem(StatusCode::UNPROCESSABLE_ENTITY, "validation");
        assert_eq!(answer.body["errors"][0]["field"], field, "{query}");
    }
    for query in ["?role=boss", "?page=first"] {
        server
            .list(&nour, query)
            .await
            .assert_problem(StatusCode::BAD_REQUEST, "malformed-query");
    }
}

#[sqlx::test]
async fn documents_the_administration_endpoints(pool: PgPool) {
    let server = Server::new(&pool);

    let document = server.send(Method::GET, "/openapi.json", None, None).await;

    let paths = &document.body["paths"];
    let operations = [
        ("/api/v1/users", "get"),
        ("/api/v1/users/{userId}", "delete"),
        ("/api/v1/users/{userId}/role", "put"),
        ("/api/v1/users/{userId}/password", "put"),
        ("/api/v1/users/{userId}/password-resets", "post"),
    ];
    for (path, method) in operations {
        let operation = &paths[path][method];
        assert_eq!(operation["tags"], json!(["Users"]), "{method} {path}");
        assert_eq!(
            operation["security"],
            json!([{ "SessionCookie": [] }]),
            "{method} {path}"
        );
        assert!(operation["responses"]["403"].is_object(), "{method} {path}");
    }
    let parameters = paths["/api/v1/users"]["get"]["parameters"]
        .as_array()
        .unwrap();
    let names: Vec<_> = parameters
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["page", "perPage", "q", "role"]);
    assert_eq!(
        paths["/api/v1/users/{userId}"]["delete"]["parameters"][0]["name"],
        "userId"
    );
}
