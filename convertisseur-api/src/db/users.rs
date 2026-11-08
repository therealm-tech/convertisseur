use sqlx::PgExecutor;
use uuid::Uuid;

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
