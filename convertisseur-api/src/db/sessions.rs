use std::time::Duration;

use sqlx::PgExecutor;
use uuid::Uuid;

use crate::users::User;

pub async fn insert(
    exec: impl PgExecutor<'_>,
    token_hash: &[u8],
    user_id: Uuid,
    ttl: Duration,
) -> sqlx::Result<()> {
    sqlx::query(include_str!("../../queries/sessions/insert.sql"))
        .bind(token_hash)
        .bind(user_id)
        .bind(ttl.as_secs_f64())
        .execute(exec)
        .await?;
    Ok(())
}

/// Pushes a live session's expiry `ttl` from now and returns its user.
pub async fn touch(
    exec: impl PgExecutor<'_>,
    token_hash: &[u8],
    ttl: Duration,
) -> sqlx::Result<Option<User>> {
    sqlx::query_as(include_str!("../../queries/sessions/touch.sql"))
        .bind(token_hash)
        .bind(ttl.as_secs_f64())
        .fetch_optional(exec)
        .await
}

pub async fn delete(exec: impl PgExecutor<'_>, token_hash: &[u8]) -> sqlx::Result<u64> {
    let result = sqlx::query(include_str!("../../queries/sessions/delete.sql"))
        .bind(token_hash)
        .execute(exec)
        .await?;
    Ok(result.rows_affected())
}

pub async fn delete_for_user(exec: impl PgExecutor<'_>, user_id: Uuid) -> sqlx::Result<u64> {
    let result = sqlx::query(include_str!("../../queries/sessions/delete_for_user.sql"))
        .bind(user_id)
        .execute(exec)
        .await?;
    Ok(result.rows_affected())
}

pub async fn delete_expired(exec: impl PgExecutor<'_>) -> sqlx::Result<u64> {
    let result = sqlx::query(include_str!("../../queries/sessions/delete_expired.sql"))
        .execute(exec)
        .await?;
    Ok(result.rows_affected())
}
