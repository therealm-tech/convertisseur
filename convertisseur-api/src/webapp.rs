use std::path::Path;

use axum::{
    Router,
    extract::Request,
    http::{HeaderValue, StatusCode, header::CACHE_CONTROL},
    middleware::{self, Next},
    response::Response,
};
use tower_http::services::{ServeDir, ServeFile};

const ASSETS: &str = "/assets";

/// Serves the built webapp. A path that matches no file gets `index.html`, so
/// that the client-side router resolves it, except under `/assets`, whose
/// missing files answer 404.
pub fn service(dir: &Path) -> Router {
    let index = dir.join("index.html");
    if index.is_file() {
        tracing::debug!(dir = %dir.display(), "serving the webapp");
    } else {
        tracing::warn!(dir = %dir.display(), "no index.html in the webapp directory: pages answer 404");
    }
    Router::new()
        .nest_service(ASSETS, ServeDir::new(dir.join("assets")))
        .fallback_service(ServeDir::new(dir).fallback(ServeFile::new(index)))
        .layer(middleware::from_fn(cache_control))
}

/// Vite names the files under `/assets` after their content, so they never
/// change; every other file keeps its name across builds and is revalidated.
async fn cache_control(request: Request, next: Next) -> Response {
    let hashed = request
        .uri()
        .path()
        .strip_prefix(ASSETS)
        .is_some_and(|rest| rest.starts_with('/'));
    let mut response = next.run(request).await;
    let found = response.status().is_success() || response.status() == StatusCode::NOT_MODIFIED;
    let policy = if hashed && found {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static(policy));
    response
}
