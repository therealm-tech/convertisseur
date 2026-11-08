use aide::{
    axum::{
        ApiRouter,
        routing::{post_with, put_with},
    },
    transform::TransformOperation,
};
use axum::{
    Json,
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, NoContent, Response},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

use super::{
    SESSION_SECURITY,
    extract::{Admin, ApiPath, CurrentSession, Login, RateLimited, RequireRole, ValidJson},
    users::{UserPath, validate_password},
};
use crate::{accounts, error::ApiError, router::AppState};

pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route(
            "/api/v1/users/{userId}/password",
            put_with(change_password, change_password_docs),
        )
        .api_route(
            "/api/v1/users/{userId}/password-resets",
            post_with(reset_password, reset_password_docs),
        )
}

/// The current password, and the one to replace it.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "currentPassword": "un mot de passe assez long",
    "newPassword": "un autre mot de passe assez long",
})))]
pub struct ChangePasswordRequest {
    current_password: String,
    /// At least 12 characters, and not the current password.
    #[validate(custom(function = validate_password))]
    new_password: String,
}

async fn change_password(
    State(state): State<AppState>,
    session: CurrentSession,
    _: RateLimited<Login>,
    ApiPath(path): ApiPath<UserPath>,
    ValidJson(request): ValidJson<ChangePasswordRequest>,
) -> Result<NoContent, ApiError> {
    if path.user_id != session.user.id {
        tracing::info!(user_id = %session.user.id, target_id = %path.user_id, "password change refused: another account");
        return Err(ApiError::Forbidden);
    }
    accounts::change_own_password(
        &state.pool,
        session.user.id,
        &session.token,
        request.current_password,
        request.new_password,
    )
    .await?;
    Ok(NoContent)
}

fn change_password_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag("Users")
        .summary("Change your password")
        .description(
            "For the account of the session only. Closes the account's other sessions and keeps \
             this one. It is the one endpoint, with the session's own, that an account whose \
             password was reset can call. Attempts count against the login rate limit.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<204, NoContent, _>(|r| r.description("Password changed"))
        .response_with::<400, ApiError, _>(|r| {
            r.description("The body is not JSON of the expected shape")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the path names another account")
        })
        .response_with::<415, ApiError, _>(|r| r.description("The body is not `application/json`"))
        .response_with::<422, ApiError, _>(|r| {
            r.description(
                "A field breaks a rule: `password_incorrect` on `currentPassword`, \
                 `password_length` or `password_unchanged` on `newPassword`",
            )
        })
        .response_with::<429, ApiError, _>(|r| {
            r.description("Too many attempts from this IP; see `Retry-After`")
        })
}

/// A temporary password, shown this once.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "temporaryPassword": "k7mqx-4tn9c-wr2hp-a8vjd",
})))]
pub struct PasswordResetResponse {
    /// To hand over to the account's owner, who must replace it at their next
    /// login. The server keeps only its hash.
    temporary_password: String,
}

async fn reset_password(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ApiPath(path): ApiPath<UserPath>,
) -> Result<Response, ApiError> {
    let temporary_password = accounts::reset_password(&state.pool, &actor, path.user_id).await?;
    Ok((
        StatusCode::CREATED,
        [(header::CACHE_CONTROL, HeaderValue::from_static("no-store"))],
        Json(PasswordResetResponse { temporary_password }),
    )
        .into_response())
}

fn reset_password_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag("Users")
        .summary("Reset an account's password")
        .description(
            "Gives the account a random temporary password, closes its sessions, and makes it \
             change that password at its next login: until then, every endpoint but the \
             password change, the current session and logout answers a \
             `password-change-required` problem. An `admin` resets `user` accounts; the \
             `super_admin` resets `user` and `admin` accounts. Nobody resets the `super_admin` \
             or their own account here. Takes no body.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<201, Json<PasswordResetResponse>, _>(|r| {
            r.description("The temporary password, which no other response repeats")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description(
                "`forbidden`: the account may not reset this one; `password-change-required`: \
                 it must change its password first",
            )
        })
        .response_with::<404, ApiError, _>(|r| r.description("No account has this id"))
}
