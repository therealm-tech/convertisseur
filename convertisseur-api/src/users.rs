use std::ops::RangeInclusive;

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Usernames chosen at registration; the configured super admin's is exempt.
pub const USERNAME_CHARS: RangeInclusive<usize> = 3..=32;

#[must_use]
pub fn is_username_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')
}

/// Roles in increasing order of power, which `Ord` follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
pub enum Role {
    User,
    Admin,
    SuperAdmin,
}

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub role: Role,
    /// Set by a password reset: until the account picks a new password, it
    /// can do nothing else.
    pub must_change_password: bool,
}

/// An account as the login sees it.
#[derive(sqlx::FromRow)]
pub struct Credentials {
    #[sqlx(flatten)]
    pub user: User,
    pub password_hash: String,
}

/// An account as the admin area lists it.
#[derive(sqlx::FromRow)]
pub struct ListedUser {
    pub id: Uuid,
    pub username: String,
    pub role: Role,
    pub created_at: DateTime<Utc>,
    pub last_active_at: Option<DateTime<Utc>>,
}

/// What the admin area can do to an account.
#[derive(Clone, Copy, Debug)]
pub enum AdminAction {
    ResetPassword,
    Delete,
    ChangeRole,
}

/// Whether `actor` may do `action` to `target` from the admin area.
///
/// The super admin exists only through the configuration, and nobody acts
/// on their own account from there. Beyond that, an account manages only
/// accounts of a lower role, and only the super admin changes roles.
#[must_use]
pub fn may(actor: &User, action: AdminAction, target: &User) -> bool {
    if target.role == Role::SuperAdmin || target.id == actor.id {
        return false;
    }
    match action {
        AdminAction::ResetPassword | AdminAction::Delete => actor.role > target.role,
        AdminAction::ChangeRole => actor.role == Role::SuperAdmin,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use uuid::Uuid;

    use super::{AdminAction, Role, User, may};

    fn account(role: Role) -> User {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        User {
            id: Uuid::from_u128(NEXT_ID.fetch_add(1, Ordering::Relaxed).into()),
            username: format!("{role:?}"),
            role,
            must_change_password: false,
        }
    }

    #[test]
    fn follows_the_permission_table() {
        use AdminAction::{ChangeRole, Delete, ResetPassword};
        use Role::{Admin, SuperAdmin, User};
        let cases = [
            (User, ResetPassword, User, false),
            (Admin, ResetPassword, User, true),
            (Admin, Delete, User, true),
            (Admin, ResetPassword, Admin, false),
            (Admin, Delete, Admin, false),
            (Admin, ChangeRole, User, false),
            (Admin, ChangeRole, Admin, false),
            (SuperAdmin, ResetPassword, User, true),
            (SuperAdmin, Delete, User, true),
            (SuperAdmin, ResetPassword, Admin, true),
            (SuperAdmin, Delete, Admin, true),
            (SuperAdmin, ChangeRole, User, true),
            (SuperAdmin, ChangeRole, Admin, true),
        ];
        for (actor, action, target, allowed) in cases {
            assert_eq!(
                may(&account(actor), action, &account(target)),
                allowed,
                "{actor:?} {action:?} {target:?}"
            );
        }
    }

    #[test]
    fn nobody_touches_the_super_admin_or_themselves() {
        let super_admin = account(Role::SuperAdmin);
        let admin = account(Role::Admin);
        for action in [
            AdminAction::ResetPassword,
            AdminAction::Delete,
            AdminAction::ChangeRole,
        ] {
            assert!(!may(&super_admin, action, &super_admin), "{action:?}");
            assert!(!may(&admin, action, &super_admin), "{action:?}");
            assert!(!may(&admin, action, &admin), "{action:?}");
        }
    }
}
