mod content;
mod extract;
mod passwords;
mod sessions;
mod tracks;
mod users;

use std::sync::Arc;

use aide::{
    axum::ApiRouter,
    openapi::{ApiKeyLocation, Info, OpenApi, SecurityScheme, Tag},
};
use axum::{
    Extension, Json, Router,
    response::Html,
    routing::{any, get},
};
use indexmap::IndexMap;

use crate::{error::ApiError, router::AppState, sessions::COOKIE_NAME};

/// Name of the session cookie's security scheme in the `OpenAPI` document.
const SESSION_SECURITY: &str = "SessionCookie";

/// The JSON API under `/api`, its `OpenAPI` document at `/openapi.json` and
/// the Scalar reference at `/docs`. Any other path under `/api` answers a
/// 404 problem rather than the webapp.
pub fn routes(state: AppState) -> Router {
    aide::generate::on_error(|error| tracing::warn!(%error, "OpenAPI generation error"));
    aide::generate::extract_schemas(true);
    // Every operation lists its responses itself: inference would document
    // the plain-text rejections of axum's extractors, which the API never
    // sends.
    aide::generate::infer_responses(false);

    let mut api = OpenApi {
        info: Info {
            title: "Convertisseur API".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            description: Some(
                "Clients must ignore unknown fields. Requests with a body must send it as \
                 `application/json`."
                    .to_owned(),
            ),
            ..Info::default()
        },
        ..OpenApi::default()
    };

    ApiRouter::new()
        .merge(sessions::routes())
        .merge(users::routes())
        .merge(passwords::routes())
        .merge(tracks::routes())
        .merge(content::routes())
        .finish_api_with(&mut api, |doc| {
            doc.tag(Tag {
                name: "Sessions".to_owned(),
                description: Some("Logging in and out, with a session cookie".to_owned()),
                ..Tag::default()
            })
            .tag(Tag {
                name: "Users".to_owned(),
                description: Some("Accounts, their passwords and their roles".to_owned()),
                ..Tag::default()
            })
            .tag(Tag {
                name: tracks::TAG.to_owned(),
                description: Some(
                    "What learners study: the validated units, lessons and cards".to_owned(),
                ),
                ..Tag::default()
            })
            .tag(Tag {
                name: content::TAG.to_owned(),
                description: Some(
                    "Units, lessons and cards of the two tracks, and their review".to_owned(),
                ),
                ..Tag::default()
            })
            .security_scheme(
                SESSION_SECURITY,
                SecurityScheme::ApiKey {
                    location: ApiKeyLocation::Cookie,
                    name: COOKIE_NAME.to_owned(),
                    description: Some("Set by `POST /api/v1/sessions`".to_owned()),
                    extensions: IndexMap::default(),
                },
            )
        })
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
