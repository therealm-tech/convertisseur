use anyhow::{Context, bail};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{db, password};

const MIN_PASSWORD_CHARS: usize = 12;

/// Makes the configured account the one and only super admin.
///
/// In one transaction, it demotes every other super admin to admin, then
/// creates the account or resets its password and role. Usernames compare
/// case-insensitively. Returns the account's id.
///
/// # Errors
///
/// Fails when the username is blank, the password is too short, or the
/// database cannot be written.
pub async fn ensure_super_admin(
    pool: &PgPool,
    username: &str,
    password: String,
) -> anyhow::Result<Uuid> {
    let username = username.trim();
    if username.is_empty() {
        bail!("the super admin username must not be blank");
    }
    if password.chars().count() < MIN_PASSWORD_CHARS {
        bail!("the super admin password must be at least {MIN_PASSWORD_CHARS} characters long");
    }
    let hash = password::hash(password).await?;

    let mut tx = pool.begin().await?;
    db::users::lock_super_admin(&mut *tx).await?;
    let demoted = db::users::demote_other_super_admins(&mut *tx, username)
        .await
        .context("failed to demote the other super admins")?;
    let id = db::users::upsert_super_admin(&mut *tx, username, &hash)
        .await
        .context("failed to upsert the super admin")?;
    tx.commit().await?;

    tracing::info!(user_id = %id, username, demoted, "super admin account ready");
    Ok(id)
}
