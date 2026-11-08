use std::ops::RangeInclusive;

use uuid::Uuid;

/// Usernames chosen at registration; the configured super admin's is exempt.
pub const USERNAME_CHARS: RangeInclusive<usize> = 3..=32;

#[must_use]
pub fn is_username_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, sqlx::Type)]
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
}

/// An account as the login sees it.
#[derive(sqlx::FromRow)]
pub struct Credentials {
    #[sqlx(flatten)]
    pub user: User,
    pub password_hash: String,
}
