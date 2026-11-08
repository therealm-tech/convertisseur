use aide::{axum::ApiRouter, axum::routing::post_with, transform::TransformOperation};
use axum::{Json, extract::State, http::StatusCode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use super::extract::{RateLimited, Registration, ValidJson};
use crate::{
    db,
    error::ApiError,
    password,
    router::AppState,
    users::{self, Role, USERNAME_CHARS, User},
};

pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new().api_route("/api/v1/users", post_with(create_user, create_user_docs))
}

/// A new account.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "username": "aicha.b",
    "password": "un mot de passe assez long",
})))]
pub struct CreateUserRequest {
    /// 3 to 32 characters among ASCII letters, digits, `.`, `_` and `-`;
    /// unique whatever its case.
    #[validate(custom(function = validate_username))]
    username: String,
    /// At least 12 characters.
    #[validate(custom(function = validate_password))]
    password: String,
}

fn validate_username(username: &str) -> Result<(), ValidationError> {
    if !USERNAME_CHARS.contains(&username.chars().count()) {
        return Err(ValidationError::new("username_length"));
    }
    if !username.chars().all(users::is_username_char) {
        return Err(ValidationError::new("username_characters"));
    }
    Ok(())
}

fn validate_password(password: &str) -> Result<(), ValidationError> {
    if password::is_long_enough(password) {
        Ok(())
    } else {
        Err(ValidationError::new("password_length"))
    }
}

/// An account.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    id: Uuid,
    username: String,
    role: RoleResponse,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoleResponse {
    User,
    Admin,
    SuperAdmin,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            role: match user.role {
                Role::User => RoleResponse::User,
                Role::Admin => RoleResponse::Admin,
                Role::SuperAdmin => RoleResponse::SuperAdmin,
            },
        }
    }
}

async fn create_user(
    State(state): State<AppState>,
    _: RateLimited<Registration>,
    ValidJson(request): ValidJson<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    let hash = password::hash(request.password).await?;
    let user = db::users::insert(&state.pool, &request.username, &hash)
        .await
        .map_err(|error| match error.as_database_error() {
            Some(db_error) if db_error.is_unique_violation() => {
                tracing::debug!(
                    username = request.username,
                    "registration refused: username taken"
                );
                ApiError::UsernameTaken
            }
            _ => error.into(),
        })?;
    tracing::info!(user_id = %user.id, username = user.username, "account created");
    Ok((StatusCode::CREATED, Json(user.into())))
}

fn create_user_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag("Users")
        .summary("Register")
        .description(
            "Creates an account with the `user` role. It does not log in: \
             open a session with `POST /api/v1/sessions` next.",
        )
        .response_with::<201, Json<UserResponse>, _>(|r| r.description("The account"))
        .response_with::<400, ApiError, _>(|r| {
            r.description("The body is not JSON of the expected shape")
        })
        .response_with::<409, ApiError, _>(|r| {
            r.description("The username is taken, whatever its case")
        })
        .response_with::<415, ApiError, _>(|r| r.description("The body is not `application/json`"))
        .response_with::<422, ApiError, _>(|r| {
            r.description(
                "A field breaks a rule: `username_length`, `username_characters` or \
                 `password_length`",
            )
        })
        .response_with::<429, ApiError, _>(|r| {
            r.description("Too many registrations from this IP; see `Retry-After`")
        })
}
