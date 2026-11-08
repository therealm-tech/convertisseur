use anyhow::{Context, bail};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    db,
    password::{self, MIN_PASSWORD_CHARS},
};

/// Makes the configured account the one and only super admin.
///
/// In one transaction, it demotes every other super admin to admin, then
/// creates the account or resets its password and role. Usernames compare
/// case-insensitively. When the password changes, the account's sessions
/// are closed. Returns the account's id.
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
    if !password::is_long_enough(&password) {
        bail!("the super admin password must be at least {MIN_PASSWORD_CHARS} characters long");
    }

    let mut tx = pool.begin().await?;
    db::users::lock_super_admin(&mut *tx).await?;
    let demoted = db::users::demote_other_super_admins(&mut *tx, username)
        .await
        .context("failed to demote the other super admins")?;
    let existing = db::users::find_credentials(&mut *tx, username).await?;
    let unchanged_hash = match existing {
        Some(account)
            if password::verify(password.clone(), account.password_hash.clone()).await? =>
        {
            Some(account.password_hash)
        }
        _ => None,
    };
    let password_changed = unchanged_hash.is_none();
    let hash = match unchanged_hash {
        Some(hash) => hash,
        None => password::hash(password).await?,
    };
    let id = db::users::upsert_super_admin(&mut *tx, username, &hash)
        .await
        .context("failed to upsert the super admin")?;
    let revoked = if password_changed {
        db::sessions::delete_for_user(&mut *tx, id)
            .await
            .context("failed to close the super admin's sessions")?
    } else {
        0
    };
    tx.commit().await?;

    tracing::info!(user_id = %id, username, demoted, password_changed, revoked, "super admin account ready");
    Ok(id)
}
