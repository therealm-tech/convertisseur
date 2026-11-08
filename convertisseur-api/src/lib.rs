//! Library half of the `convertisseur-api` binary: the router, its state and
//! the start-up bootstrap live here so that the integration tests can drive
//! them without a socket.

mod accounts;
mod api;
mod bootstrap;
pub mod config;
mod content;
mod db;
mod error;
mod health;
mod password;
mod rate_limit;
mod router;
mod sessions;
pub mod storage;
mod users;
mod webapp;

pub use bootstrap::ensure_super_admin;
pub use router::{AppState, ClientSettings, app};
pub use sessions::COOKIE_NAME;

/// The schema migrations, embedded at compile time.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();
