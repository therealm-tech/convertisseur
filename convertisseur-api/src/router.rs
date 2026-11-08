use std::{path::Path, sync::Arc};

use axum::{Router, http::HeaderName, routing::get};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

use crate::{api, db, health, rate_limit::RateLimits, webapp};

/// How the server sees its HTTP clients.
#[derive(Clone)]
pub struct ClientSettings {
    /// Whether the session cookie carries `Secure`, so that it only travels
    /// over HTTPS.
    pub cookie_secure: bool,
    /// The header in which a trusted reverse proxy passes the client's IP;
    /// without it, the client is the peer of the TCP connection.
    pub client_ip_header: Option<HeaderName>,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            cookie_secure: true,
            client_ip_header: None,
        }
    }
}

/// Shared state handed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub(crate) pool: PgPool,
    pub(crate) clients: Arc<ClientSettings>,
    pub(crate) rate_limits: Arc<RateLimits>,
}

impl AppState {
    #[must_use]
    pub fn new(pool: PgPool, clients: ClientSettings) -> Self {
        Self {
            pool,
            clients: Arc::new(clients),
            rate_limits: Arc::new(RateLimits::default()),
        }
    }

    /// Deletes the expired sessions and forgets the clients the rate limits
    /// no longer hold back. Returns how many sessions it deleted.
    ///
    /// # Errors
    ///
    /// Fails when the database cannot be written.
    pub async fn housekeeping(&self) -> sqlx::Result<u64> {
        self.rate_limits.forget_idle_clients();
        let purged = db::sessions::delete_expired(&self.pool).await?;
        tracing::debug!(purged, "expired sessions purged");
        Ok(purged)
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
