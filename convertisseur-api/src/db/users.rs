use sqlx::PgExecutor;
use uuid::Uuid;

use crate::users::{Credentials, User};

/// Key of the transaction-level advisory lock that serialises the super
/// admin bootstrap of replicas starting at the same time.
const SUPER_ADMIN_LOCK_KEY: i64 = 0x636f_6e76_7375_7061;

pub async fn lock_super_admin(exec: impl PgExecutor<'_>) -> sqlx::Result<()> {
    sqlx::query(include_str!("../../queries/users/lock_super_admin.sql"))
        .bind(SUPER_ADMIN_LOCK_KEY)
        .execute(exec)
        .await?;
    Ok(())
}

/// Demotes to admin every super admin but `username`; returns how many.
pub async fn demote_other_super_admins(
    exec: impl PgExecutor<'_>,
    username: &str,
) -> sqlx::Result<u64> {
    let result = sqlx::query(include_str!(
        "../../queries/users/demote_other_super_admins.sql"
    ))
    .bind(username)
    .execute(exec)
    .await?;
    Ok(result.rows_affected())
}

pub async fn upsert_super_admin(
    exec: impl PgExecutor<'_>,
    username: &str,
    password_hash: &str,
) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(include_str!("../../queries/users/upsert_super_admin.sql"))
        .bind(username)
        .bind(password_hash)
        .fetch_one(exec)
        .await
}

/// Creates a `user` account; fails with a unique violation when the
/// username is taken, whatever its case.
pub async fn insert(
    exec: impl PgExecutor<'_>,
    username: &str,
    password_hash: &str,
) -> sqlx::Result<User> {
    sqlx::query_as(include_str!("../../queries/users/insert.sql"))
        .bind(username)
        .bind(password_hash)
        .fetch_one(exec)
        .await
}

/// The account named `username`, compared case-insensitively.
pub async fn find_credentials(
    exec: impl PgExecutor<'_>,
    username: &str,
) -> sqlx::Result<Option<Credentials>> {
    sqlx::query_as(include_str!("../../queries/users/find_credentials.sql"))
        .bind(username)
        .fetch_optional(exec)
        .await
}
