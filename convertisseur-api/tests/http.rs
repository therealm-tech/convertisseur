use std::{fs, path::Path};

use axum::{
    Router,
    body::Body,
    http::{
        Request, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE, IF_MODIFIED_SINCE, LAST_MODIFIED},
    },
};
use convertisseur_api::{AppState, ClientSettings, app};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tempfile::TempDir;
use tower::ServiceExt;

const INDEX: &str = "<!doctype html><title>Convertisseur</title>";

fn webapp_dir() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("index.html"), INDEX).unwrap();
    for file in [
        "sw.js",
        "registerSW.js",
        "manifest.webmanifest",
        "favicon.ico",
    ] {
        fs::write(dir.path().join(file), file).unwrap();
    }
    fs::create_dir(dir.path().join("assets")).unwrap();
    fs::write(dir.path().join("assets/app.js"), "console.log(1)").unwrap();
    dir
}

fn router(pool: PgPool, webapp_dir: &Path) -> Router {
    app(AppState::new(pool, ClientSettings::default()), webapp_dir)
}

struct Answer {
    status: StatusCode,
    content_type: Option<String>,
    cache_control: Option<String>,
    body: String,
}

async fn get(router: &Router, uri: &str) -> Answer {
    let response = router
        .clone()
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let header = |name| {
        response
            .headers()
            .get(name)
            .map(|v: &axum::http::HeaderValue| v.to_str().unwrap().to_owned())
    };
    let content_type = header(CONTENT_TYPE);
    let cache_control = header(CACHE_CONTROL);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    Answer {
        status,
        content_type,
        cache_control,
        body: String::from_utf8(bytes.to_vec()).unwrap(),
    }
}

#[sqlx::test]
async fn liveness_answers_ok(pool: PgPool) {
    let dir = webapp_dir();
    let answer = get(&router(pool, dir.path()), "/healthz").await;
    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.body, "ok");
}

#[sqlx::test]
async fn readiness_follows_the_database(pool: PgPool) {
    let dir = webapp_dir();
    let router = router(pool.clone(), dir.path());

    assert_eq!(get(&router, "/readyz").await.status, StatusCode::OK);

    pool.close().await;
    let answer = get(&router, "/readyz").await;
    assert_eq!(answer.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(get(&router, "/healthz").await.status, StatusCode::OK);
}

#[sqlx::test]
async fn serves_the_webapp_with_an_index_fallback(pool: PgPool) {
    let dir = webapp_dir();
    let router = router(pool, dir.path());

    let root = get(&router, "/").await;
    assert_eq!(root.status, StatusCode::OK);
    assert_eq!(root.body, INDEX);

    let asset = get(&router, "/assets/app.js").await;
    assert_eq!(asset.status, StatusCode::OK);
    assert_eq!(asset.body, "console.log(1)");

    let deep_link = get(&router, "/lessons/arabic").await;
    assert_eq!(deep_link.status, StatusCode::OK);
    assert_eq!(deep_link.body, INDEX);
}

#[sqlx::test]
async fn hashed_assets_are_immutable_and_the_rest_is_revalidated(pool: PgPool) {
    let dir = webapp_dir();
    let router = router(pool, dir.path());

    let asset = get(&router, "/assets/app.js").await;
    assert_eq!(asset.status, StatusCode::OK);
    assert_eq!(
        asset.cache_control.as_deref(),
        Some("public, max-age=31536000, immutable")
    );

    for uri in [
        "/",
        "/index.html",
        "/lessons/arabic",
        "/sw.js",
        "/registerSW.js",
        "/manifest.webmanifest",
        "/favicon.ico",
    ] {
        let answer = get(&router, uri).await;
        assert_eq!(answer.status, StatusCode::OK, "{uri}");
        assert_eq!(answer.cache_control.as_deref(), Some("no-cache"), "{uri}");
    }
}

#[sqlx::test]
async fn a_revalidated_asset_stays_immutable(pool: PgPool) {
    let dir = webapp_dir();
    let router = router(pool, dir.path());
    let last_modified = router
        .clone()
        .oneshot(Request::get("/assets/app.js").body(Body::empty()).unwrap())
        .await
        .unwrap()
        .headers()[LAST_MODIFIED]
        .clone();

    let response = router
        .oneshot(
            Request::get("/assets/app.js")
                .header(IF_MODIFIED_SINCE, last_modified)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(
        response.headers()[CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
}

#[sqlx::test]
async fn missing_assets_answer_404_rather_than_the_index(pool: PgPool) {
    let dir = webapp_dir();
    let answer = get(&router(pool, dir.path()), "/assets/gone-1a2b3c.js").await;
    assert_eq!(answer.status, StatusCode::NOT_FOUND);
    assert_eq!(answer.cache_control.as_deref(), Some("no-cache"));
}

#[sqlx::test]
async fn unknown_api_paths_answer_a_problem(pool: PgPool) {
    let dir = webapp_dir();
    let router = router(pool, dir.path());

    for uri in ["/api", "/api/v1/nothing-here"] {
        let answer = get(&router, uri).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(
            answer.content_type.as_deref(),
            Some("application/problem+json")
        );
        let problem: serde_json::Value = serde_json::from_str(&answer.body).unwrap();
        assert_eq!(problem["status"], 404);
        assert_eq!(problem["type"], "urn:convertisseur:problem:not-found");
    }
}

#[sqlx::test]
async fn publishes_the_openapi_document_and_its_reference(pool: PgPool) {
    let dir = webapp_dir();
    let router = router(pool, dir.path());

    let document = get(&router, "/openapi.json").await;
    assert_eq!(document.status, StatusCode::OK);
    let document: serde_json::Value = serde_json::from_str(&document.body).unwrap();
    assert_eq!(document["info"]["title"], "Convertisseur API");

    let docs = get(&router, "/docs").await;
    assert_eq!(docs.status, StatusCode::OK);
    assert!(docs.body.contains("@scalar/api-reference@"));
}
