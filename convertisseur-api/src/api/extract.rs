//! The request extractors the API handlers share: a JSON body, a query and a
//! path that must be valid, the rate limits on the client's IP, the live
//! session, and the role guard.

use std::{
    marker::PhantomData,
    net::{IpAddr, SocketAddr},
};

use aide::{OperationInput, generate::GenContext, openapi::Operation};
use axum::{
    Json,
    extract::{
        ConnectInfo, FromRequest, FromRequestParts, Path, Query, Request, rejection::JsonRejection,
    },
    http::{HeaderMap, HeaderName, request::Parts},
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::{
    error::ApiError,
    rate_limit::Bucket,
    router::AppState,
    sessions::{self, SessionToken},
    users::{Role, User},
};

/// A JSON body, deserialised then validated. Any other content type is
/// refused, which keeps cross-site HTML forms from reaching the handler.
pub struct ValidJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|rejection| {
                tracing::debug!(%rejection, "request body refused");
                match rejection {
                    JsonRejection::MissingJsonContentType(_) => ApiError::UnsupportedMediaType,
                    _ => ApiError::MalformedBody,
                }
            })?;
        value.validate()?;
        Ok(Self(value))
    }
}

impl<T: JsonSchema> OperationInput for ValidJson<T> {
    fn operation_input(ctx: &mut GenContext, operation: &mut Operation) {
        Json::<T>::operation_input(ctx, operation);
    }
}

/// Query parameters, deserialised then validated.
pub struct ValidQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for ValidQuery<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) =
            Query::<T>::from_request_parts(parts, state)
                .await
                .map_err(|rejection| {
                    tracing::debug!(%rejection, "query refused");
                    ApiError::MalformedQuery
                })?;
        value.validate()?;
        Ok(Self(value))
    }
}

impl<T: JsonSchema> OperationInput for ValidQuery<T> {
    fn operation_input(ctx: &mut GenContext, operation: &mut Operation) {
        Query::<T>::operation_input(ctx, operation);
    }
}

/// Path parameters. One that does not parse names nothing, so it answers a
/// 404 problem.
pub struct ApiPath<T>(pub T);

impl<T, S> FromRequestParts<S> for ApiPath<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value) =
            Path::<T>::from_request_parts(parts, state)
                .await
                .map_err(|rejection| {
                    tracing::debug!(%rejection, "path refused");
                    ApiError::NotFound
                })?;
        Ok(Self(value))
    }
}

impl<T: JsonSchema> OperationInput for ApiPath<T> {
    fn operation_input(ctx: &mut GenContext, operation: &mut Operation) {
        Path::<T>::operation_input(ctx, operation);
    }
}

/// The client's IP: the last address of the configured proxy header when
/// the request carries one, the TCP peer otherwise.
async fn client_ip(parts: &mut Parts, state: &AppState) -> Result<IpAddr, ApiError> {
    if let Some(name) = &state.clients.client_ip_header {
        if let Some(ip) = forwarded_ip(&parts.headers, name) {
            return Ok(ip);
        }
        tracing::debug!(header = %name, "no usable client IP header, falling back to the peer");
    }
    let ConnectInfo(peer) = ConnectInfo::<SocketAddr>::from_request_parts(parts, state)
        .await
        .map_err(|rejection| ApiError::Internal(anyhow::anyhow!("unknown peer: {rejection}")))?;
    Ok(peer.ip())
}

fn forwarded_ip(headers: &HeaderMap, name: &HeaderName) -> Option<IpAddr> {
    let value = headers.get_all(name).iter().next_back()?.to_str().ok()?;
    value.rsplit(',').next()?.trim().parse().ok()
}

/// Which rate limit an endpoint counts against.
pub trait Limited {
    const BUCKET: Bucket;
}

pub struct Login;

impl Limited for Login {
    const BUCKET: Bucket = Bucket::Login;
}

pub struct Registration;

impl Limited for Registration {
    const BUCKET: Bucket = Bucket::Registration;
}

/// Counts the request against the client IP's limit for `L`, and refuses it
/// with a 429 once spent, before the body is even read.
pub struct RateLimited<L>(PhantomData<L>);

impl<L: Limited + Send> FromRequestParts<AppState> for RateLimited<L> {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let ip = client_ip(parts, state).await?;
        state
            .rate_limits
            .check(L::BUCKET, ip)
            .map_err(|retry_after| {
                tracing::warn!(%ip, bucket = ?L::BUCKET, ?retry_after, "rate limit reached");
                ApiError::RateLimited { retry_after }
            })?;
        Ok(Self(PhantomData))
    }
}

impl<L> OperationInput for RateLimited<L> {}

/// The live session the request's cookie names, slid forward by this use.
/// Refuses the request with a 401 when there is none. It lets through an
/// account that must change its password: only the endpoints it needs to do
/// so take it, every other one takes [`Authenticated`].
pub struct CurrentSession {
    pub token: SessionToken,
    pub user: User,
}

impl FromRequestParts<AppState> for CurrentSession {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = SessionToken::from_headers(&parts.headers).ok_or(ApiError::Unauthenticated)?;
        let user = sessions::resume(&state.pool, &token)
            .await?
            .ok_or(ApiError::Unauthenticated)?;
        Ok(Self { token, user })
    }
}

impl OperationInput for CurrentSession {}

/// The account of a live session that has no pending password change.
/// Refuses the request with a 401 without a session, and with a 403
/// `password-change-required` problem while a reset password is unchanged.
pub struct Authenticated(pub User);

impl FromRequestParts<AppState> for Authenticated {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let CurrentSession { user, .. } = CurrentSession::from_request_parts(parts, state).await?;
        if user.must_change_password {
            tracing::debug!(user_id = %user.id, "refused until the password is changed");
            return Err(ApiError::PasswordChangeRequired);
        }
        Ok(Self(user))
    }
}

impl OperationInput for Authenticated {}

/// The least role an endpoint needs.
pub trait MinimumRole {
    const ROLE: Role;
}

pub struct Admin;

impl MinimumRole for Admin {
    const ROLE: Role = Role::Admin;
}

pub struct SuperAdmin;

impl MinimumRole for SuperAdmin {
    const ROLE: Role = Role::SuperAdmin;
}

/// An [`Authenticated`] account whose role is at least `R`'s. Refuses any
/// other with a 403 `forbidden` problem.
pub struct RequireRole<R> {
    pub user: User,
    role: PhantomData<R>,
}

impl<R: MinimumRole + Send> FromRequestParts<AppState> for RequireRole<R> {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Authenticated(user) = Authenticated::from_request_parts(parts, state).await?;
        if user.role < R::ROLE {
            tracing::info!(user_id = %user.id, role = ?user.role, needed = ?R::ROLE, "role too low");
            return Err(ApiError::Forbidden);
        }
        Ok(Self {
            user,
            role: PhantomData,
        })
    }
}

impl<R> OperationInput for RequireRole<R> {}
