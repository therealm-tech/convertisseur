use aide::{
    axum::{
        ApiRouter,
        routing::{delete_with, post_with, put_with},
    },
    transform::TransformOperation,
};
use axum::{Json, extract::State, http::StatusCode, response::NoContent};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use super::{
    SESSION_SECURITY,
    extract::{
        Admin, ApiPath, Authenticated, RateLimited, Registration, RequireRole, SuperAdmin,
        ValidJson, ValidQuery,
    },
};
use crate::{
    accounts, db,
    error::ApiError,
    password,
    router::AppState,
    users::{self, ListedUser, Role, USERNAME_CHARS, User},
};

const DEFAULT_PER_PAGE: u32 = 50;
const MAX_PER_PAGE: u32 = 100;

pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route(
            "/api/v1/users",
            post_with(create_user, create_user_docs).get_with(list_users, list_users_docs),
        )
        .api_route(
            "/api/v1/users/{userId}",
            delete_with(delete_user, delete_user_docs),
        )
        .api_route(
            "/api/v1/users/{userId}/role",
            put_with(set_role, set_role_docs),
        )
        .api_route(
            "/api/v1/users/{userId}/email",
            put_with(set_email, set_email_docs),
        )
}

/// Names one account in the path.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserPath {
    pub user_id: Uuid,
}

/// A new account.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "username": "aicha.b",
    "email": "aicha@example.com",
    "password": "un mot de passe assez long",
})))]
pub struct CreateUserRequest {
    /// 3 to 32 characters among ASCII letters, digits, `.`, `_` and `-`;
    /// unique whatever its case.
    #[validate(custom(function = validate_username))]
    username: String,
    /// An email address, unique whatever its case. Nothing is sent to it.
    #[validate(email(code = "email_invalid"))]
    email: String,
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

/// Maps a unique violation on the username or the email to its problem.
fn taken(error: sqlx::Error) -> ApiError {
    match error.as_database_error().and_then(|e| e.constraint()) {
        Some(db::users::USERNAME_KEY) => ApiError::UsernameTaken,
        Some(db::users::EMAIL_KEY) => ApiError::EmailTaken,
        _ => error.into(),
    }
}

pub(super) fn validate_password(password: &str) -> Result<(), ValidationError> {
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
    /// `null` for an account created before emails were required, until it
    /// sets one.
    email: Option<String>,
    role: RoleResponse,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoleResponse {
    User,
    Admin,
    SuperAdmin,
}

impl From<Role> for RoleResponse {
    fn from(role: Role) -> Self {
        match role {
            Role::User => Self::User,
            Role::Admin => Self::Admin,
            Role::SuperAdmin => Self::SuperAdmin,
        }
    }
}

#[derive(PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoleRequest {
    User,
    Admin,
    SuperAdmin,
}

impl From<RoleRequest> for Role {
    fn from(role: RoleRequest) -> Self {
        match role {
            RoleRequest::User => Self::User,
            RoleRequest::Admin => Self::Admin,
            RoleRequest::SuperAdmin => Self::SuperAdmin,
        }
    }
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            role: user.role.into(),
        }
    }
}

async fn create_user(
    State(state): State<AppState>,
    _: RateLimited<Registration>,
    ValidJson(request): ValidJson<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    let hash = password::hash(request.password).await?;
    let user = db::users::insert(&state.pool, &request.username, &request.email, &hash)
        .await
        .map_err(|error| {
            let error = taken(error);
            if matches!(error, ApiError::UsernameTaken | ApiError::EmailTaken) {
                tracing::debug!(username = request.username, ?error, "registration refused");
            }
            error
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
            r.description(
                "`username-taken` or `email-taken`: another account has it, whatever its case",
            )
        })
        .response_with::<415, ApiError, _>(|r| r.description("The body is not `application/json`"))
        .response_with::<422, ApiError, _>(|r| {
            r.description(
                "A field breaks a rule: `username_length`, `username_characters`, \
                 `email_invalid` or `password_length`",
            )
        })
        .response_with::<429, ApiError, _>(|r| {
            r.description("Too many registrations from this IP; see `Retry-After`")
        })
}

/// Which accounts to list, and which page of them.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ListUsersQuery {
    /// 1-based; 1 by default.
    #[validate(range(min = 1))]
    page: Option<u32>,
    /// 1 to 100; 50 by default.
    #[validate(range(min = 1, max = MAX_PER_PAGE))]
    per_page: Option<u32>,
    /// Part of the username or of the email, compared case-insensitively.
    q: Option<String>,
    /// Only the accounts with this role.
    role: Option<RoleRequest>,
}

/// One page of accounts: the super admin, then the admins, then the users,
/// each by username.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserPageResponse {
    items: Vec<UserListItemResponse>,
    page: u32,
    per_page: u32,
    /// How many accounts match the filters, across all pages.
    total: u64,
}

/// An account, as the admin area lists it.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserListItemResponse {
    id: Uuid,
    username: String,
    /// `null` until an account created before emails were required sets one.
    email: Option<String>,
    role: RoleResponse,
    created_at: DateTime<Utc>,
    /// The last time one of its sessions was used; `null` if it never logged
    /// in.
    last_active_at: Option<DateTime<Utc>>,
}

impl From<ListedUser> for UserListItemResponse {
    fn from(user: ListedUser) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            role: user.role.into(),
            created_at: user.created_at,
            last_active_at: user.last_active_at,
        }
    }
}

async fn list_users(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ValidQuery(query): ValidQuery<ListUsersQuery>,
) -> Result<Json<UserPageResponse>, ApiError> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(DEFAULT_PER_PAGE);
    let filter = db::users::UserFilter {
        text_contains: query.q.as_deref().map(str::trim).filter(|q| !q.is_empty()),
        role: query.role.map(Role::from),
    };
    let offset = i64::from(page - 1) * i64::from(per_page);
    let total = db::users::count(&state.pool, &filter).await?;
    let items = db::users::list(&state.pool, &filter, per_page.into(), offset).await?;
    tracing::debug!(actor_id = %actor.id, page, per_page, total, "accounts listed");
    Ok(Json(UserPageResponse {
        items: items.into_iter().map(Into::into).collect(),
        page,
        per_page,
        total: total.try_into().unwrap_or_default(),
    }))
}

fn list_users_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag("Users")
        .summary("List accounts")
        .description(
            "For an `admin` or the `super_admin`, the only accounts that see others' emails.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<UserPageResponse>, _>(|r| r.description("One page of accounts"))
        .response_with::<400, ApiError, _>(|r| {
            r.description("A query parameter is not of the expected type")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description(
                "`forbidden`: the account is a `user`; `password-change-required`: it must \
                 change its password first",
            )
        })
        .response_with::<422, ApiError, _>(|r| r.description("`page` or `perPage` is out of range"))
}

async fn delete_user(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ApiPath(path): ApiPath<UserPath>,
) -> Result<NoContent, ApiError> {
    accounts::delete(&state.pool, &actor, path.user_id).await?;
    Ok(NoContent)
}

fn delete_user_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag("Users")
        .summary("Delete an account")
        .description(
            "Deletes the account and closes its sessions. An `admin` deletes `user` accounts; \
             the `super_admin` deletes `user` and `admin` accounts. Nobody deletes the \
             `super_admin` or their own account. Answers 204 when the account is already gone, \
             so that a retry is harmless.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<204, NoContent, _>(|r| r.description("The account is gone"))
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description(
                "`forbidden`: the account may not delete this one; `password-change-required`: \
                 it must change its password first",
            )
        })
}

/// The role to give an account.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({ "role": "admin" })))]
pub struct SetRoleRequest {
    /// `user` or `admin`: the `super_admin` comes from the server's
    /// configuration.
    #[validate(custom(function = validate_assignable_role))]
    role: RoleRequest,
}

fn validate_assignable_role(role: &RoleRequest) -> Result<(), ValidationError> {
    if role == &RoleRequest::SuperAdmin {
        Err(ValidationError::new("role_not_assignable"))
    } else {
        Ok(())
    }
}

/// An account's role.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoleAssignmentResponse {
    role: RoleResponse,
}

async fn set_role(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<SuperAdmin>,
    ApiPath(path): ApiPath<UserPath>,
    ValidJson(request): ValidJson<SetRoleRequest>,
) -> Result<Json<RoleAssignmentResponse>, ApiError> {
    let role = Role::from(request.role);
    accounts::set_role(&state.pool, &actor, path.user_id, role).await?;
    Ok(Json(RoleAssignmentResponse { role: role.into() }))
}

fn set_role_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag("Users")
        .summary("Set an account's role")
        .description(
            "For the `super_admin` only, on any account but their own. The account's sessions \
             stay open and carry the new role from their next request.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<RoleAssignmentResponse>, _>(|r| r.description("The new role"))
        .response_with::<400, ApiError, _>(|r| {
            r.description("The body is not JSON of the expected shape")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description(
                "`forbidden`: the account is not the `super_admin`, or the target is itself; \
                 `password-change-required`: it must change its password first",
            )
        })
        .response_with::<404, ApiError, _>(|r| r.description("No account has this id"))
        .response_with::<415, ApiError, _>(|r| r.description("The body is not `application/json`"))
        .response_with::<422, ApiError, _>(|r| {
            r.description("`role_not_assignable`: the role is `super_admin`")
        })
}

/// The email to log in with.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({ "email": "aicha@example.com" })))]
pub struct SetEmailRequest {
    /// An email address, unique whatever its case. Nothing is sent to it.
    #[validate(email(code = "email_invalid"))]
    email: String,
}

/// An account's email.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EmailResponse {
    email: String,
}

async fn set_email(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    ApiPath(path): ApiPath<UserPath>,
    ValidJson(request): ValidJson<SetEmailRequest>,
) -> Result<Json<EmailResponse>, ApiError> {
    if path.user_id != user.id {
        tracing::info!(user_id = %user.id, target_id = %path.user_id, "email change refused: another account");
        return Err(ApiError::Forbidden);
    }
    db::users::set_email(&state.pool, user.id, &request.email)
        .await
        .map_err(|error| {
            let error = taken(error);
            if matches!(error, ApiError::EmailTaken) {
                tracing::debug!(user_id = %user.id, "email change refused: taken");
            }
            error
        })?;
    tracing::info!(user_id = %user.id, "email changed");
    Ok(Json(EmailResponse {
        email: request.email,
    }))
}

fn set_email_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag("Users")
        .summary("Set your email")
        .description(
            "For the account of the session only, the `super_admin` included. The email logs in \
             in place of the username from then on. It is not verified, and nothing is sent to \
             it.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<EmailResponse>, _>(|r| r.description("The new email"))
        .response_with::<400, ApiError, _>(|r| {
            r.description("The body is not JSON of the expected shape")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description(
                "`forbidden`: the path names another account; `password-change-required`: it \
                 must change its password first",
            )
        })
        .response_with::<409, ApiError, _>(|r| {
            r.description("`email-taken`: another account has it, whatever its case")
        })
        .response_with::<415, ApiError, _>(|r| r.description("The body is not `application/json`"))
        .response_with::<422, ApiError, _>(|r| {
            r.description("`email_invalid`: not an email address")
        })
}
