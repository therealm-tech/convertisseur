use std::sync::Arc;

use aide::{
    axum::ApiRouter,
    openapi::{Info, OpenApi},
};
use axum::{
    Extension, Json, Router,
    response::Html,
    routing::{any, get},
};

use crate::{error::ApiError, router::AppState};

/// The JSON API under `/api`, its `OpenAPI` document at `/openapi.json` and
/// the Scalar reference at `/docs`. Any other path under `/api` answers a
/// 404 problem rather than the webapp.
pub fn routes(state: AppState) -> Router {
    aide::generate::on_error(|error| tracing::warn!(%error, "OpenAPI generation error"));
    aide::generate::extract_schemas(true);

    let mut api = OpenApi {
        info: Info {
            title: "Convertisseur API".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            description: Some("Clients must ignore unknown fields.".to_owned()),
            ..Info::default()
        },
        ..OpenApi::default()
    };

    ApiRouter::new()
        .finish_api(&mut api)
        .with_state(state)
        .route("/openapi.json", get(openapi))
        .route("/docs", get(docs))
        .route("/api", any(not_found))
        .route("/api/{*path}", any(not_found))
        .layer(Extension(Arc::new(api)))
}

async fn openapi(Extension(api): Extension<Arc<OpenApi>>) -> Json<Arc<OpenApi>> {
    Json(api)
}

async fn docs() -> Html<&'static str> {
    Html(include_str!("api/docs.html"))
}

async fn not_found() -> ApiError {
    ApiError::NotFound
}
