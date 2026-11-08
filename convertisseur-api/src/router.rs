use std::path::Path;

use axum::{Router, routing::get};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

use crate::{api, health, webapp};

/// Shared state handed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub(crate) pool: PgPool,
}

impl AppState {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Builds the whole HTTP application: the API and its documentation, the
/// `/healthz` and `/readyz` probes, and the webapp for every other path.
pub fn app(state: AppState, webapp_dir: &Path) -> Router {
    Router::new()
        .route("/healthz", get(health::liveness))
        .route("/readyz", get(health::readiness))
        .with_state(state.clone())
        .merge(api::routes(state))
        .fallback_service(webapp::service(webapp_dir))
        .layer(TraceLayer::new_for_http())
}
