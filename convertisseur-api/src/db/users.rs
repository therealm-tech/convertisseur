use sqlx::PgExecutor;
use uuid::Uuid;

use crate::users::{Credentials, ListedUser, Role, User};

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

/// The id of the super admin, when the server has created it.
pub async fn find_super_admin(exec: impl PgExecutor<'_>) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar(include_str!("../../queries/users/find_super_admin.sql"))
        .fetch_optional(exec)
        .await
}

/// Index behind the uniqueness of usernames, whatever their case.
pub const USERNAME_KEY: &str = "users_username_key";
/// Index behind the uniqueness of emails, whatever their case.
pub const EMAIL_KEY: &str = "users_email_key";

/// Creates a `user` account; fails with a unique violation on
/// [`USERNAME_KEY`] or [`EMAIL_KEY`] when either is taken.
pub async fn insert(
    exec: impl PgExecutor<'_>,
    username: &str,
    email: &str,
    password_hash: &str,
) -> sqlx::Result<User> {
    sqlx::query_as(include_str!("../../queries/users/insert.sql"))
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .fetch_one(exec)
        .await
}

/// The account whose username or email is `login`, compared
/// case-insensitively; the username wins when both match.
pub async fn find_credentials(
    exec: impl PgExecutor<'_>,
    login: &str,
) -> sqlx::Result<Option<Credentials>> {
    sqlx::query_as(include_str!("../../queries/users/find_credentials.sql"))
        .bind(login)
        .fetch_optional(exec)
        .await
}

pub async fn find_credentials_by_id(
    exec: impl PgExecutor<'_>,
    id: Uuid,
) -> sqlx::Result<Option<Credentials>> {
    sqlx::query_as(include_str!(
        "../../queries/users/find_credentials_by_id.sql"
    ))
    .bind(id)
    .fetch_optional(exec)
    .await
}

/// The account `id`, locked until the end of the transaction so that its
/// role cannot change between a permission check and the action.
pub async fn lock(exec: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<Option<User>> {
    sqlx::query_as(include_str!("../../queries/users/lock.sql"))
        .bind(id)
        .fetch_optional(exec)
        .await
}

/// Which accounts a listing returns: `None` matches everything.
pub struct UserFilter<'a> {
    /// Part of the username or of the email, compared case-insensitively.
    pub text_contains: Option<&'a str>,
    pub role: Option<Role>,
}

pub async fn count(exec: impl PgExecutor<'_>, filter: &UserFilter<'_>) -> sqlx::Result<i64> {
    sqlx::query_scalar(include_str!("../../queries/users/count.sql"))
        .bind(filter.text_contains)
        .bind(filter.role)
        .fetch_one(exec)
        .await
}

/// One page of the accounts matching `filter`: the super admin, then the
/// admins, then the users, each by username.
pub async fn list(
    exec: impl PgExecutor<'_>,
    filter: &UserFilter<'_>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<ListedUser>> {
    sqlx::query_as(include_str!("../../queries/users/list.sql"))
        .bind(filter.text_contains)
        .bind(filter.role)
        .bind(limit)
        .bind(offset)
        .fetch_all(exec)
        .await
}

pub async fn set_role(exec: impl PgExecutor<'_>, id: Uuid, role: Role) -> sqlx::Result<()> {
    sqlx::query(include_str!("../../queries/users/set_role.sql"))
        .bind(id)
        .bind(role)
        .execute(exec)
        .await?;
    Ok(())
}

/// Stores a temporary password, which the account must change at its next
/// login.
pub async fn set_temporary_password(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    password_hash: &str,
) -> sqlx::Result<()> {
    sqlx::query(include_str!(
        "../../queries/users/set_temporary_password.sql"
    ))
    .bind(id)
    .bind(password_hash)
    .execute(exec)
    .await?;
    Ok(())
}

/// Fails with a unique violation on [`EMAIL_KEY`] when another account has
/// `email`, whatever its case.
pub async fn set_email(exec: impl PgExecutor<'_>, id: Uuid, email: &str) -> sqlx::Result<()> {
    sqlx::query(include_str!("../../queries/users/set_email.sql"))
        .bind(id)
        .bind(email)
        .execute(exec)
        .await?;
    Ok(())
}

pub async fn delete(exec: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<u64> {
    let result = sqlx::query(include_str!("../../queries/users/delete.sql"))
        .bind(id)
        .execute(exec)
        .await?;
    Ok(result.rows_affected())
}

/// Swaps the password hash `old_hash` for `new_hash` and clears a pending
/// forced change; returns `false`, changing nothing, when the stored hash is
/// no longer `old_hash`.
pub async fn replace_password(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    old_hash: &str,
    new_hash: &str,
) -> sqlx::Result<bool> {
    let result = sqlx::query(include_str!("../../queries/users/replace_password.sql"))
        .bind(id)
        .bind(old_hash)
        .bind(new_hash)
        .execute(exec)
        .await?;
    Ok(result.rows_affected() == 1)
}

/// The usernames of the accounts among `ids` that still exist.
pub async fn usernames(
    exec: impl PgExecutor<'_>,
    ids: &[Uuid],
) -> sqlx::Result<Vec<(Uuid, String)>> {
    sqlx::query_as(include_str!("../../queries/users/usernames.sql"))
        .bind(ids)
        .fetch_all(exec)
        .await
}
