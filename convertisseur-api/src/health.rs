use std::{fmt::Display, time::Duration};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{router::AppState, storage::ObjectStore};

const READINESS_TIMEOUT: Duration = Duration::from_secs(2);

/// The process is up and serving HTTP.
pub async fn liveness() -> &'static str {
    "ok"
}

/// The process can serve requests: the database and the object store answer.
pub async fn readiness(State(state): State<AppState>) -> Response {
    let (database, storage) = tokio::join!(
        answers("database", sqlx::query("SELECT 1").execute(&state.pool)),
        answers("object store", state.storage.check()),
    );
    let down: Vec<_> = [(database, "database"), (storage, "object store")]
        .into_iter()
        .filter_map(|(up, name)| (!up).then_some(name))
        .collect();
    if down.is_empty() {
        (StatusCode::OK, "ok").into_response()
    } else {
        let body = format!("{} unavailable", down.join(", "));
        (StatusCode::SERVICE_UNAVAILABLE, body).into_response()
    }
}

async fn answers<T, E: Display>(
    dependency: &'static str,
    probe: impl Future<Output = Result<T, E>>,
) -> bool {
    match tokio::time::timeout(READINESS_TIMEOUT, probe).await {
        Ok(Ok(_)) => {
            tracing::debug!(dependency, "readiness probe: answered");
            true
        }
        Ok(Err(error)) => {
            tracing::warn!(
                dependency,
                error = format!("{error:#}"),
                "readiness probe: unreachable"
            );
            false
        }
        Err(_) => {
            tracing::warn!(dependency, timeout = ?READINESS_TIMEOUT, "readiness probe: timed out");
            false
        }
    }
}
