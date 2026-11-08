//! Library half of the `convertisseur-api` binary: the router, its state and
//! the start-up bootstrap live here so that the integration tests can drive
//! them without a socket.

mod api;
mod bootstrap;
pub mod config;
mod db;
mod error;
mod health;
mod password;
mod router;
mod webapp;

pub use bootstrap::ensure_super_admin;
pub use router::{AppState, app};

/// The schema migrations, embedded at compile time.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();
