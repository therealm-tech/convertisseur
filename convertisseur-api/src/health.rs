use std::time::Duration;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::router::AppState;

const READINESS_TIMEOUT: Duration = Duration::from_secs(2);

/// The process is up and serving HTTP.
pub async fn liveness() -> &'static str {
    "ok"
}

/// The process can serve requests: the database answers.
pub async fn readiness(State(state): State<AppState>) -> Response {
    let ping = sqlx::query("SELECT 1").execute(&state.pool);
    match tokio::time::timeout(READINESS_TIMEOUT, ping).await {
        Ok(Ok(_)) => {
            tracing::debug!("readiness probe: database answered");
            (StatusCode::OK, "ok").into_response()
        }
        Ok(Err(error)) => {
            tracing::warn!(%error, "readiness probe: database unreachable");
            (StatusCode::SERVICE_UNAVAILABLE, "database unavailable").into_response()
        }
        Err(_) => {
            tracing::warn!(timeout = ?READINESS_TIMEOUT, "readiness probe: database timed out");
            (StatusCode::SERVICE_UNAVAILABLE, "database unavailable").into_response()
        }
    }
}
