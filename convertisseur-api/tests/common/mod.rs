//! The harness the API test files share: the app behind an in-process
//! router, its answers, and accounts made straight in the database. A
//! `common/mod.rs` rather than `common.rs`, which Cargo would build as a test
//! file of its own.
// Each test file uses its own subset.
#![allow(dead_code)]

use std::{net::SocketAddr, sync::OnceLock};

use argon2::{Argon2, PasswordHasher};
use axum::{
    Router,
    body::Body,
    extract::connect_info::MockConnectInfo,
    http::{HeaderMap, Method, Request, StatusCode, header},
};
use convertisseur_api::{
    AppState, ClientSettings, app,
    storage::{MemoryStore, Storage},
    sync::{Catalogue, SyncReport, sync},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, PgPool};
use tower::ServiceExt;
use uuid::Uuid;

pub const PASSWORD: &str = "a long enough password";
const PEER: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 40000);

pub struct Answer {
    pub status: StatusCode,
    pub headers: HeaderMap,
    /// The body as JSON, or as text when it is not JSON.
    pub body: Value,
    pub bytes: Vec<u8>,
}

impl Answer {
    pub fn problem_type(&self) -> &str {
        assert_eq!(
            self.headers.get(header::CONTENT_TYPE).unwrap(),
            "application/problem+json"
        );
        self.body["type"].as_str().unwrap()
    }

    pub fn assert_problem(&self, status: StatusCode, kind: &str) {
        assert_eq!(self.status, status, "{}", self.body);
        assert_eq!(
            self.problem_type(),
            format!("urn:convertisseur:problem:{kind}")
        );
    }

    /// Asserts a validation problem whose only broken rule is `code` on
    /// `field`.
    pub fn assert_invalid(&self, field: &str, code: &str) {
        self.assert_problem(StatusCode::UNPROCESSABLE_ENTITY, "validation");
        assert_eq!(
            self.body["errors"],
            json!([{ "field": field, "code": code }]),
            "{}",
            self.body
        );
    }

    pub fn cookie(&self) -> String {
        let set_cookie = self.headers[header::SET_COOKIE].to_str().unwrap();
        set_cookie.split(';').next().unwrap().to_owned()
    }
}

/// The app, with the login rate limit of a single client: a test logs in at
/// most 10 times per `Server`.
pub struct Server {
    router: Router,
}

impl Server {
    pub fn new(pool: &PgPool) -> Self {
        Self::with_storage(pool, Storage::Memory(MemoryStore::default()))
    }

    pub fn with_storage(pool: &PgPool, storage: Storage) -> Self {
        let state = AppState::new(pool.clone(), storage, ClientSettings::default());
        let router = app(state, &std::env::temp_dir()).layer(MockConnectInfo(PEER));
        Self { router }
    }

    pub async fn request(&self, request: Request<Body>) -> Answer {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
        };
        Answer {
            status,
            headers,
            body,
            bytes: bytes.to_vec(),
        }
    }

    pub async fn send(
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
        self.request(request.unwrap()).await
    }

    pub async fn login(&self, login: &str, password: &str) -> Answer {
        let body = json!({ "login": login, "password": password });
        self.send(Method::POST, "/api/v1/sessions", None, Some(&body))
            .await
    }

    /// Logs in as `username`, with [`PASSWORD`]; returns the session cookie.
    pub async fn session(&self, username: &str) -> String {
        let answer = self.login(username, PASSWORD).await;
        assert_eq!(answer.status, StatusCode::CREATED, "{username}");
        answer.cookie()
    }
}

/// Hashing is slow in a debug build: every account a test creates shares
/// this hash of [`PASSWORD`].
pub fn password_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| {
        Argon2::default()
            .hash_password(PASSWORD.as_bytes())
            .unwrap()
            .to_string()
    })
}

pub async fn create_account(pool: &PgPool, username: &str, role: &str) -> Uuid {
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

/// A content file of one unit, two lessons and four cards, synced in place
/// of the built-in ones.
pub const SALUTATIONS: &str = "
key: salutations
title: Salutations
variety: darija
lessons:
  - key: bonjour
    title: Bonjour
    cards:
      - key: salam
        arabic: سَلَام
        transliteration: salam
        meaning_fr: Paix
      - key: sbah
        arabic: صْبَاح الْخِير
        transliteration: sbah el khir
        meaning_fr: Bonjour, le matin
      - key: msa
        arabic: مْسَا الْخِير
        transliteration: msa el khir
        meaning_fr: Bonsoir
  - key: politesse
    title: Politesse
    cards:
      - key: choukran
        arabic: شُكْرًا
        transliteration: choukran
        meaning_fr: Merci
";

/// Syncs `files`, given as `(path under content/, text)`, keeping the
/// objects of recordings in `store`.
pub async fn sync_files(
    pool: &PgPool,
    store: &MemoryStore,
    files: &[(&str, &str)],
) -> anyhow::Result<SyncReport> {
    let catalogue = Catalogue::parse(files).unwrap_or_else(|invalid| panic!("{invalid}"));
    sync(pool, &Storage::Memory(store.clone()), &catalogue).await
}

/// Syncs `text` as the only content file, `arabe/01-salutations.yaml`.
pub async fn sync_salutations(
    pool: &PgPool,
    store: &MemoryStore,
    text: &str,
) -> anyhow::Result<SyncReport> {
    sync_files(pool, store, &[("arabe/01-salutations.yaml", text)]).await
}

/// The id of the row of `table` imported under `key`.
pub async fn imported_id(pool: &PgPool, table: &str, key: &str) -> Uuid {
    sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT id FROM {table} WHERE import_key = $1"
    )))
    .bind(key)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|error| panic!("{table} {key}: {error}"))
}
