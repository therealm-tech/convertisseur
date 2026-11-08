use std::net::SocketAddr;

use axum::{
    Router,
    body::Body,
    extract::connect_info::MockConnectInfo,
    http::{HeaderMap, HeaderName, Method, Request, StatusCode, header},
};
use convertisseur_api::{AppState, COOKIE_NAME, ClientSettings, app, ensure_super_admin};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

const PASSWORD: &str = "a long enough password";
const PEER: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 40000);

struct Server {
    router: Router,
}

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

    fn set_cookie(&self) -> &str {
        self.headers
            .get(header::SET_COOKIE)
            .expect("a Set-Cookie header")
            .to_str()
            .unwrap()
    }

    /// The `name=value` pair to send back in a `Cookie` header.
    fn cookie(&self) -> String {
        self.set_cookie().split(';').next().unwrap().to_owned()
    }
}

impl Server {
    fn new(pool: PgPool) -> Self {
        Self::with(pool, ClientSettings::default())
    }

    fn with(pool: PgPool, clients: ClientSettings) -> Self {
        let dir = std::env::temp_dir();
        let router = app(AppState::new(pool, clients), &dir).layer(MockConnectInfo(PEER));
        Self { router }
    }

    async fn send(&self, request: Request<Body>) -> Answer {
        let response = self.router.clone().oneshot(request).await.unwrap();
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

    async fn post(&self, uri: &str, body: &Value) -> Answer {
        self.send(json_request(Method::POST, uri, body)).await
    }

    async fn register(&self, username: &str, password: &str) -> Answer {
        self.post(
            "/api/v1/users",
            &json!({ "username": username, "password": password }),
        )
        .await
    }

    async fn login(&self, username: &str, password: &str) -> Answer {
        self.post(
            "/api/v1/sessions",
            &json!({ "username": username, "password": password }),
        )
        .await
    }

    async fn current_session(&self, cookie: Option<&str>) -> Answer {
        self.send(with_cookie(Method::GET, "/api/v1/sessions/current", cookie))
            .await
    }

    async fn logout(&self, cookie: Option<&str>) -> Answer {
        self.send(with_cookie(
            Method::DELETE,
            "/api/v1/sessions/current",
            cookie,
        ))
        .await
    }

    /// Registers `username` and logs in; returns the session cookie.
    async fn session_for(&self, username: &str) -> String {
        assert_eq!(
            self.register(username, PASSWORD).await.status,
            StatusCode::CREATED
        );
        let login = self.login(username, PASSWORD).await;
        assert_eq!(login.status, StatusCode::CREATED);
        login.cookie()
    }
}

fn json_request(method: Method, uri: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn with_cookie(method: Method, uri: &str, cookie: Option<&str>) -> Request<Body> {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    request.body(Body::empty()).unwrap()
}

async fn session_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM sessions")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn registers_a_user(pool: PgPool) {
    let server = Server::new(pool.clone());

    let answer = server.register("aicha.b", PASSWORD).await;

    assert_eq!(answer.status, StatusCode::CREATED);
    assert_eq!(answer.body["username"], "aicha.b");
    assert_eq!(answer.body["role"], "user");
    let (id, hash): (uuid::Uuid, String) =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE username = 'aicha.b'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(answer.body["id"], id.to_string());
    assert!(hash.starts_with("$argon2id$"));
    assert_eq!(session_count(&pool).await, 0, "registering does not log in");
}

#[sqlx::test]
async fn refuses_a_username_taken_in_another_case(pool: PgPool) {
    let server = Server::new(pool);
    assert_eq!(
        server.register("Aicha", PASSWORD).await.status,
        StatusCode::CREATED
    );

    let answer = server.register("aICHA", PASSWORD).await;

    assert_eq!(answer.status, StatusCode::CONFLICT);
    assert_eq!(
        answer.problem_type(),
        "urn:convertisseur:problem:username-taken"
    );
}

#[sqlx::test]
async fn refuses_the_super_admin_username_as_any_other_taken_one(pool: PgPool) {
    ensure_super_admin(&pool, "admin", PASSWORD.to_owned())
        .await
        .unwrap();
    let server = Server::new(pool);

    let answer = server.register("Admin", PASSWORD).await;

    assert_eq!(answer.status, StatusCode::CONFLICT);
    assert_eq!(
        answer.problem_type(),
        "urn:convertisseur:problem:username-taken"
    );
}

#[sqlx::test]
async fn refuses_an_invalid_username_or_password(pool: PgPool) {
    let server = Server::new(pool.clone());
    let cases = [
        ("ab", PASSWORD, "username", "username_length"),
        (&"a".repeat(33), PASSWORD, "username", "username_length"),
        ("aïcha", PASSWORD, "username", "username_characters"),
        ("aicha b", PASSWORD, "username", "username_characters"),
        ("aicha", "elevenchars", "password", "password_length"),
    ];

    for (username, password, field, code) in cases {
        let answer = server.register(username, password).await;
        assert_eq!(
            answer.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{username}"
        );
        assert_eq!(
            answer.problem_type(),
            "urn:convertisseur:problem:validation"
        );
        assert_eq!(
            answer.body["errors"],
            json!([{ "field": field, "code": code }])
        );
    }
    let users: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(users, 0);
}

#[sqlx::test]
async fn accepts_the_username_bounds(pool: PgPool) {
    let server = Server::new(pool);
    for username in ["a.b", "A_b-9.z", &"z".repeat(32)] {
        assert_eq!(
            server.register(username, PASSWORD).await.status,
            StatusCode::CREATED,
            "{username}"
        );
    }
}

#[sqlx::test]
async fn refuses_a_body_that_is_not_json(pool: PgPool) {
    let server = Server::new(pool);

    let form = Request::post("/api/v1/sessions")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(format!("username=aicha&password={PASSWORD}")))
        .unwrap();
    let answer = server.send(form).await;
    assert_eq!(answer.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(
        answer.problem_type(),
        "urn:convertisseur:problem:unsupported-media-type"
    );

    let broken = Request::post("/api/v1/users")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{\"username\":"))
        .unwrap();
    let answer = server.send(broken).await;
    assert_eq!(answer.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        answer.problem_type(),
        "urn:convertisseur:problem:malformed-body"
    );
}

#[sqlx::test]
async fn logs_in_with_a_secure_session_cookie(pool: PgPool) {
    let server = Server::new(pool.clone());
    server.register("aicha", PASSWORD).await;

    let answer = server.login("AICHA", PASSWORD).await;

    assert_eq!(answer.status, StatusCode::CREATED);
    assert_eq!(answer.body["user"]["username"], "aicha");
    assert_eq!(answer.body["user"]["role"], "user");
    assert_eq!(
        answer.headers.get(header::LOCATION).unwrap(),
        "/api/v1/sessions/current"
    );
    let set_cookie = answer.set_cookie();
    assert!(set_cookie.starts_with(&format!("{COOKIE_NAME}=")));
    for attribute in [
        "HttpOnly",
        "SameSite=Lax",
        "Secure",
        "Path=/",
        "Max-Age=2592000",
    ] {
        assert!(
            set_cookie.contains(attribute),
            "{attribute} in {set_cookie}"
        );
    }

    let token = answer.cookie().split_once('=').unwrap().1.to_owned();
    let stored: Vec<u8> = sqlx::query_scalar("SELECT token_hash FROM sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored.len(), 32, "a SHA-256");
    assert_ne!(stored, token.as_bytes(), "the token itself is never stored");
}

#[sqlx::test]
async fn drops_secure_from_the_cookie_when_configured(pool: PgPool) {
    let server = Server::with(
        pool,
        ClientSettings {
            cookie_secure: false,
            client_ip_header: None,
        },
    );
    server.register("aicha", PASSWORD).await;

    let answer = server.login("aicha", PASSWORD).await;

    assert_eq!(answer.status, StatusCode::CREATED);
    assert!(!answer.set_cookie().contains("Secure"));
}

#[sqlx::test]
async fn a_wrong_password_and_an_unknown_user_look_the_same(pool: PgPool) {
    let server = Server::new(pool.clone());
    server.register("aicha", PASSWORD).await;

    let wrong_password = server.login("aicha", "not the right password").await;
    let unknown_user = server.login("nobody", PASSWORD).await;

    assert_eq!(wrong_password.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        wrong_password.problem_type(),
        "urn:convertisseur:problem:invalid-credentials"
    );
    assert_eq!(unknown_user.status, wrong_password.status);
    assert_eq!(unknown_user.body, wrong_password.body);
    assert!(wrong_password.headers.get(header::SET_COOKIE).is_none());
    assert_eq!(session_count(&pool).await, 0);
}

#[sqlx::test]
async fn the_current_session_needs_a_live_cookie(pool: PgPool) {
    let server = Server::new(pool);

    for cookie in [
        None,
        Some(format!("{COOKIE_NAME}=forged")),
        Some("other=1".to_owned()),
    ] {
        let answer = server.current_session(cookie.as_deref()).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{cookie:?}");
        assert_eq!(
            answer.problem_type(),
            "urn:convertisseur:problem:unauthenticated"
        );
    }
}

#[sqlx::test]
async fn the_current_session_names_its_user_and_renews_the_cookie(pool: PgPool) {
    let server = Server::new(pool);
    let cookie = server.session_for("aicha").await;

    let answer = server.current_session(Some(&cookie)).await;

    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body["user"]["username"], "aicha");
    assert_eq!(answer.body["user"]["role"], "user");
    assert!(answer.body["user"]["id"].is_string());
    assert_eq!(answer.cookie(), cookie);
    assert!(answer.set_cookie().contains("Max-Age=2592000"));
}

#[sqlx::test]
async fn an_expired_session_is_refused(pool: PgPool) {
    let server = Server::new(pool.clone());
    let cookie = server.session_for("aicha").await;

    sqlx::query("UPDATE sessions SET expires_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();

    let answer = server.current_session(Some(&cookie)).await;
    assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn using_a_session_slides_its_expiry(pool: PgPool) {
    let server = Server::new(pool.clone());
    let cookie = server.session_for("aicha").await;
    sqlx::query("UPDATE sessions SET expires_at = now() + interval '1 day'")
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        server.current_session(Some(&cookie)).await.status,
        StatusCode::OK
    );

    let slid: bool =
        sqlx::query_scalar("SELECT expires_at > now() + interval '29 days' FROM sessions")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(slid);
}

#[sqlx::test]
async fn housekeeping_purges_expired_sessions(pool: PgPool) {
    let state = AppState::new(pool.clone(), ClientSettings::default());
    let router = app(state.clone(), &std::env::temp_dir()).layer(MockConnectInfo(PEER));
    let server = Server { router };
    server.session_for("aicha").await;
    server.session_for("bilal").await;
    sqlx::query(
        "UPDATE sessions SET expires_at = now() - interval '1 second' \
         FROM users WHERE users.id = sessions.user_id AND users.username = 'aicha'",
    )
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(state.housekeeping().await.unwrap(), 1);
    assert_eq!(session_count(&pool).await, 1);
}

#[sqlx::test]
async fn logging_out_closes_the_session_and_clears_the_cookie(pool: PgPool) {
    let server = Server::new(pool.clone());
    let cookie = server.session_for("aicha").await;

    let answer = server.logout(Some(&cookie)).await;

    assert_eq!(answer.status, StatusCode::NO_CONTENT);
    assert!(answer.set_cookie().starts_with(&format!("{COOKIE_NAME}=;")));
    assert!(answer.set_cookie().contains("Max-Age=0"));
    assert_eq!(session_count(&pool).await, 0);
    assert_eq!(
        server.current_session(Some(&cookie)).await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test]
async fn logging_out_without_a_session_is_harmless(pool: PgPool) {
    let server = Server::new(pool);

    assert_eq!(server.logout(None).await.status, StatusCode::NO_CONTENT);
    assert_eq!(
        server
            .logout(Some(&format!("{COOKIE_NAME}=gone")))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
}

#[sqlx::test]
async fn rate_limits_registrations_per_client_ip(pool: PgPool) {
    let server = Server::new(pool);

    for _ in 0..5 {
        assert_eq!(
            server.register("x", PASSWORD).await.status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let answer = server.register("aicha", PASSWORD).await;

    assert_eq!(answer.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        answer.problem_type(),
        "urn:convertisseur:problem:rate-limited"
    );
    let retry_after: u64 = answer.headers[header::RETRY_AFTER]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((1..=600).contains(&retry_after), "{retry_after}");
}

#[sqlx::test]
async fn rate_limits_logins_per_client_ip(pool: PgPool) {
    let server = Server::new(pool);
    let empty_body = json!({});

    for _ in 0..10 {
        assert_eq!(
            server.post("/api/v1/sessions", &empty_body).await.status,
            StatusCode::BAD_REQUEST
        );
    }
    let answer = server.login("aicha", PASSWORD).await;

    assert_eq!(answer.status, StatusCode::TOO_MANY_REQUESTS);
    assert!(answer.headers.contains_key(header::RETRY_AFTER));
}

async fn post_from(server: &Server, forwarded_for: &str) -> StatusCode {
    let mut request = json_request(Method::POST, "/api/v1/users", &json!({}));
    request
        .headers_mut()
        .insert("x-forwarded-for", forwarded_for.parse().unwrap());
    server.send(request).await.status
}

#[sqlx::test]
async fn reads_the_client_ip_from_the_trusted_proxy_header(pool: PgPool) {
    let server = Server::with(
        pool,
        ClientSettings {
            cookie_secure: true,
            client_ip_header: Some(HeaderName::from_static("x-forwarded-for")),
        },
    );

    for _ in 0..5 {
        assert_eq!(
            post_from(&server, "203.0.113.7").await,
            StatusCode::BAD_REQUEST
        );
    }
    // The proxy appends the address it saw: whatever the client wrote before
    // it does not change who is counted.
    assert_eq!(
        post_from(&server, "198.51.100.1, 203.0.113.7").await,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        post_from(&server, "203.0.113.8").await,
        StatusCode::BAD_REQUEST
    );
}

#[sqlx::test]
async fn ignores_forwarding_headers_unless_configured(pool: PgPool) {
    let server = Server::new(pool);

    for i in 0..5 {
        assert_eq!(
            post_from(&server, &format!("203.0.113.{i}")).await,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        post_from(&server, "203.0.113.99").await,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[sqlx::test]
async fn changing_the_super_admin_password_closes_its_sessions(pool: PgPool) {
    ensure_super_admin(&pool, "admin", PASSWORD.to_owned())
        .await
        .unwrap();
    let server = Server::new(pool.clone());
    let admin = server.login("admin", PASSWORD).await.cookie();
    let user = server.session_for("aicha").await;

    ensure_super_admin(&pool, "admin", PASSWORD.to_owned())
        .await
        .unwrap();
    let kept = server.current_session(Some(&admin)).await;
    assert_eq!(kept.status, StatusCode::OK, "same password, same sessions");
    assert_eq!(kept.body["user"]["role"], "super_admin");

    ensure_super_admin(&pool, "admin", "a brand new password".to_owned())
        .await
        .unwrap();
    assert_eq!(
        server.current_session(Some(&admin)).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        server.current_session(Some(&user)).await.status,
        StatusCode::OK
    );
    assert_eq!(
        server.login("admin", "a brand new password").await.status,
        StatusCode::CREATED
    );
}

#[sqlx::test]
async fn documents_the_endpoints(pool: PgPool) {
    let server = Server::new(pool);

    let document = server
        .send(Request::get("/openapi.json").body(Body::empty()).unwrap())
        .await;

    let paths = &document.body["paths"];
    assert_eq!(paths["/api/v1/users"]["post"]["tags"], json!(["Users"]));
    assert_eq!(
        paths["/api/v1/sessions"]["post"]["tags"],
        json!(["Sessions"])
    );
    let current = &paths["/api/v1/sessions/current"];
    assert!(current["get"]["responses"]["401"].is_object());
    assert!(current["delete"]["responses"]["204"].is_object());
    assert_eq!(current["get"]["security"], json!([{ "SessionCookie": [] }]));
    assert_eq!(
        document.body["components"]["securitySchemes"]["SessionCookie"],
        json!({
            "type": "apiKey",
            "in": "cookie",
            "name": COOKIE_NAME,
            "description": "Set by `POST /api/v1/sessions`",
        })
    );
    assert!(
        paths["/api/v1/users"]["post"]["responses"]["422"]["content"]["application/problem+json"]
            .is_object()
    );
}
