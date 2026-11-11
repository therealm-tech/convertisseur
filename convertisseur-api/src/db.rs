//! Persistence: one module per aggregate, each running the SQL kept under
//! `queries/`.

pub mod audio;
pub mod content;
pub mod progress;
pub mod review;
pub mod sessions;
pub mod sync;
pub mod tracks;
pub mod users;
