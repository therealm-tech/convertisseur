use sqlx::PgExecutor;
use uuid::Uuid;

use crate::content::{Audio, Playable};

/// What the server knows of a stored recording.
pub struct NewAudio<'a> {
    pub id: Uuid,
    pub object_key: &'a str,
    pub content_type: &'a str,
    pub size_bytes: i64,
    pub duration_ms: Option<i32>,
    pub created_by: Uuid,
}

pub async fn insert(exec: impl PgExecutor<'_>, audio: &NewAudio<'_>) -> sqlx::Result<Audio> {
    sqlx::query_as(include_str!("../../queries/audio/insert.sql"))
        .bind(audio.id)
        .bind(audio.object_key)
        .bind(audio.content_type)
        .bind(audio.size_bytes)
        .bind(audio.duration_ms)
        .bind(audio.created_by)
        .fetch_one(exec)
        .await
}

/// Deletes the row; returns the key of its object, which the caller deletes
/// from the store once the transaction commits.
pub async fn delete(exec: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar(include_str!("../../queries/audio/delete.sql"))
        .bind(id)
        .fetch_optional(exec)
        .await
}

pub async fn find_playable(exec: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<Option<Playable>> {
    sqlx::query_as(include_str!("../../queries/audio/find_playable.sql"))
        .bind(id)
        .fetch_optional(exec)
        .await
}
