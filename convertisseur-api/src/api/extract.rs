//! The request extractors the API handlers share: a JSON body that must be
//! valid, the rate limits on the client's IP, and the live session.

use std::{
    marker::PhantomData,
    net::{IpAddr, SocketAddr},
};

use aide::{OperationInput, generate::GenContext, openapi::Operation};
use axum::{
    Json,
    extract::{ConnectInfo, FromRequest, FromRequestParts, Request, rejection::JsonRejection},
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
    users::User,
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
/// Refuses the request with a 401 when there is none.
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
