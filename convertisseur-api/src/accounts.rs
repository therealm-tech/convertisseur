//! What an account does to itself, and what the admin area does to the
//! others, each in one transaction.

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    db, password,
    sessions::SessionToken,
    users::{AdminAction, Role, User, may},
};

/// Why an account operation did not happen.
#[derive(Debug)]
pub enum Refusal {
    /// No account has this id.
    NotFound,
    /// The actor may not do this to that account.
    Forbidden,
    /// The current password given is wrong.
    WrongPassword,
    /// The new password is the current one.
    SamePassword,
    Failed(anyhow::Error),
}

impl From<sqlx::Error> for Refusal {
    fn from(error: sqlx::Error) -> Self {
        Self::Failed(error.into())
    }
}

impl From<anyhow::Error> for Refusal {
    fn from(error: anyhow::Error) -> Self {
        Self::Failed(error)
    }
}

/// Locks `target_id` and checks that `actor` may do `action` to it.
async fn authorise(
    tx: &mut Transaction<'_, Postgres>,
    actor: &User,
    action: AdminAction,
    target_id: Uuid,
) -> Result<Option<User>, Refusal> {
    let Some(target) = db::users::lock(&mut **tx, target_id).await? else {
        tracing::debug!(actor_id = %actor.id, %target_id, ?action, "no such account");
        return Ok(None);
    };
    if !may(actor, action, &target) {
        tracing::info!(
            actor_id = %actor.id,
            actor_role = ?actor.role,
            %target_id,
            target_role = ?target.role,
            ?action,
            "admin action refused"
        );
        return Err(Refusal::Forbidden);
    }
    Ok(Some(target))
}

/// Gives `target_id` a random temporary password that it must change at its
/// next login, and closes its sessions. Returns the temporary password,
/// which is stored nowhere but as a hash.
pub async fn reset_password(
    pool: &PgPool,
    actor: &User,
    target_id: Uuid,
) -> Result<String, Refusal> {
    let mut tx = pool.begin().await?;
    authorise(&mut tx, actor, AdminAction::ResetPassword, target_id)
        .await?
        .ok_or(Refusal::NotFound)?;
    let temporary = password::generate_temporary()?;
    let hash = password::hash(temporary.clone()).await?;
    db::users::set_temporary_password(&mut *tx, target_id, &hash).await?;
    let revoked = db::sessions::delete_for_user(&mut *tx, target_id).await?;
    tx.commit().await?;
    tracing::info!(actor_id = %actor.id, %target_id, revoked, "password reset");
    Ok(temporary)
}

/// Deletes `target_id` and, through the foreign key, its sessions. An
/// account that no longer exists is not an error, so that a retry is
/// harmless.
pub async fn delete(pool: &PgPool, actor: &User, target_id: Uuid) -> Result<(), Refusal> {
    let mut tx = pool.begin().await?;
    if authorise(&mut tx, actor, AdminAction::Delete, target_id)
        .await?
        .is_some()
    {
        db::users::delete(&mut *tx, target_id).await?;
        tracing::info!(actor_id = %actor.id, %target_id, "account deleted");
    }
    tx.commit().await?;
    Ok(())
}

/// Gives `target_id` the role `user` or `admin`. Its sessions stay open:
/// every request reads the role afresh.
pub async fn set_role(
    pool: &PgPool,
    actor: &User,
    target_id: Uuid,
    role: Role,
) -> Result<(), Refusal> {
    debug_assert!(
        role != Role::SuperAdmin,
        "the super admin comes from config"
    );
    let mut tx = pool.begin().await?;
    let target = authorise(&mut tx, actor, AdminAction::ChangeRole, target_id)
        .await?
        .ok_or(Refusal::NotFound)?;
    if target.role == role {
        tracing::debug!(%target_id, ?role, "role unchanged");
    } else {
        db::users::set_role(&mut *tx, target_id, role).await?;
        tracing::info!(actor_id = %actor.id, %target_id, from = ?target.role, to = ?role, "role changed");
    }
    tx.commit().await?;
    Ok(())
}

/// Replaces the password of the session's account after checking the
/// current one, clears a pending forced change, and closes the account's
/// other sessions.
pub async fn change_own_password(
    pool: &PgPool,
    user_id: Uuid,
    session: &SessionToken,
    current: String,
    new: String,
) -> Result<(), Refusal> {
    if current == new {
        return Err(Refusal::SamePassword);
    }
    let account = db::users::find_credentials_by_id(pool, user_id)
        .await?
        .ok_or(Refusal::NotFound)?;
    if !password::verify(current, account.password_hash.clone()).await? {
        tracing::info!(%user_id, "password change refused: wrong current password");
        return Err(Refusal::WrongPassword);
    }
    let hash = password::hash(new).await?;

    let mut tx = pool.begin().await?;
    // The compare-and-swap refuses a password changed by another request
    // while this one was hashing.
    if !db::users::replace_password(&mut *tx, user_id, &account.password_hash, &hash).await? {
        tracing::info!(%user_id, "password change refused: changed concurrently");
        return Err(Refusal::WrongPassword);
    }
    let revoked = db::sessions::delete_others_for_user(&mut *tx, user_id, &session.hash()).await?;
    tx.commit().await?;
    tracing::info!(%user_id, revoked, "password changed");
    Ok(())
}
