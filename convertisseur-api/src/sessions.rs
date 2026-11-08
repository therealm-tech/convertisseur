use std::time::Duration;

use anyhow::anyhow;
use axum::http::{HeaderMap, HeaderValue, header::COOKIE};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use cookie::{Cookie, SameSite};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{db, users::User};

/// How long a session lives after its last use.
pub const SESSION_TTL: Duration = Duration::from_hours(30 * 24);

/// Named after the app: browsers share cookies across the ports of one host,
/// so a generic `session` would collide with other apps on `localhost`.
pub const COOKIE_NAME: &str = "convertisseur_session";

const TOKEN_BYTES: usize = 32;

/// The secret the browser holds. Only its SHA-256 reaches the database.
pub struct SessionToken(String);

impl SessionToken {
    fn generate() -> anyhow::Result<Self> {
        let mut bytes = [0u8; TOKEN_BYTES];
        getrandom::fill(&mut bytes).map_err(|e| anyhow!("no randomness for a session: {e}"))?;
        Ok(Self(URL_SAFE_NO_PAD.encode(bytes)))
    }

    /// The token in the request's session cookie, if any.
    #[must_use]
    pub fn from_headers(headers: &HeaderMap) -> Option<Self> {
        headers
            .get_all(COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(Cookie::split_parse)
            .filter_map(Result::ok)
            .find(|cookie| cookie.name() == COOKIE_NAME && !cookie.value().is_empty())
            .map(|cookie| Self(cookie.value().to_owned()))
    }

    pub(crate) fn hash(&self) -> Vec<u8> {
        Sha256::digest(self.0.as_bytes()).to_vec()
    }

    /// The `Set-Cookie` value that stores this token for [`SESSION_TTL`].
    #[must_use]
    pub fn cookie(&self, secure: bool) -> HeaderValue {
        let max_age = i64::try_from(SESSION_TTL.as_secs()).unwrap_or(i64::MAX);
        set_cookie(&self.0, cookie::time::Duration::seconds(max_age), secure)
    }
}

/// The `Set-Cookie` value that makes the browser drop the session cookie.
#[must_use]
pub fn removal_cookie(secure: bool) -> HeaderValue {
    set_cookie("", cookie::time::Duration::ZERO, secure)
}

fn set_cookie(value: &str, max_age: cookie::time::Duration, secure: bool) -> HeaderValue {
    let cookie = Cookie::build((COOKIE_NAME, value))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .max_age(max_age)
        .build();
    HeaderValue::try_from(cookie.to_string()).expect("a base64url token is a valid header value")
}

/// Opens a session for `user_id` and returns the token to hand the browser.
pub async fn open(pool: &PgPool, user_id: Uuid) -> anyhow::Result<SessionToken> {
    let token = SessionToken::generate()?;
    db::sessions::insert(pool, &token.hash(), user_id, SESSION_TTL).await?;
    tracing::debug!(%user_id, "session opened");
    Ok(token)
}

/// The user behind a live session, whose expiry moves [`SESSION_TTL`] ahead.
/// `None` when the token matches no session or an expired one.
pub async fn resume(pool: &PgPool, token: &SessionToken) -> sqlx::Result<Option<User>> {
    let user = db::sessions::touch(pool, &token.hash(), SESSION_TTL).await?;
    if let Some(user) = &user {
        tracing::debug!(user_id = %user.id, "session resumed");
    } else {
        tracing::debug!("no live session for the cookie");
    }
    Ok(user)
}

pub async fn close(pool: &PgPool, token: &SessionToken) -> sqlx::Result<()> {
    let closed = db::sessions::delete(pool, &token.hash()).await?;
    tracing::debug!(closed, "session closed");
    Ok(())
}
