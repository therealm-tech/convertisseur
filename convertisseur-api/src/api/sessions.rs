use aide::{
    axum::{
        ApiRouter,
        routing::{get_with, post_with},
    },
    transform::TransformOperation,
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, NoContent, Response},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

use super::{
    SESSION_SECURITY,
    extract::{CurrentSession, Login, RateLimited, ValidJson},
    users::UserResponse,
};
use crate::{
    db,
    error::ApiError,
    password,
    router::AppState,
    sessions::{self, SessionToken},
};

const CURRENT_SESSION_PATH: &str = "/api/v1/sessions/current";

pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route(
            "/api/v1/sessions",
            post_with(create_session, create_session_docs),
        )
        .api_route(
            CURRENT_SESSION_PATH,
            get_with(get_current_session, get_current_session_docs)
                .delete_with(delete_current_session, delete_current_session_docs),
        )
}

/// A username and its password.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "username": "aicha.b",
    "password": "un mot de passe assez long",
})))]
pub struct CreateSessionRequest {
    /// Compared case-insensitively.
    username: String,
    password: String,
}

/// A logged-in session.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    /// The account the session belongs to.
    user: UserResponse,
}

async fn create_session(
    State(state): State<AppState>,
    _: RateLimited<Login>,
    ValidJson(request): ValidJson<CreateSessionRequest>,
) -> Result<Response, ApiError> {
    let Some(account) = db::users::find_credentials(&state.pool, &request.username).await? else {
        password::verify_dummy(request.password).await?;
        tracing::info!(
            username = request.username,
            "login refused: unknown username"
        );
        return Err(ApiError::InvalidCredentials);
    };
    if !password::verify(request.password, account.password_hash).await? {
        tracing::info!(user_id = %account.user.id, "login refused: wrong password");
        return Err(ApiError::InvalidCredentials);
    }
    let token = sessions::open(&state.pool, account.user.id).await?;
    tracing::info!(user_id = %account.user.id, "logged in");

    let body = SessionResponse {
        user: account.user.into(),
    };
    Ok((
        StatusCode::CREATED,
        [
            (
                header::SET_COOKIE,
                token.cookie(state.clients.cookie_secure),
            ),
            (
                header::LOCATION,
                HeaderValue::from_static(CURRENT_SESSION_PATH),
            ),
        ],
        Json(body),
    )
        .into_response())
}

fn create_session_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag("Sessions")
        .summary("Log in")
        .description(
            "Opens a session and sets it in the `convertisseur_session` cookie: `HttpOnly`, \
             `SameSite=Lax`, `Secure` unless the server is configured otherwise. A session \
             expires 30 days after its last use. A wrong password and an unknown username \
             get the same answer.",
        )
        .response_with::<201, Json<SessionResponse>, _>(|r| {
            r.description("Logged in; `Set-Cookie` holds the session")
        })
        .response_with::<400, ApiError, _>(|r| {
            r.description("The body is not JSON of the expected shape")
        })
        .response_with::<401, ApiError, _>(|r| {
            r.description("The username or the password is wrong")
        })
        .response_with::<415, ApiError, _>(|r| r.description("The body is not `application/json`"))
        .response_with::<429, ApiError, _>(|r| {
            r.description("Too many attempts from this IP; see `Retry-After`")
        })
}

async fn get_current_session(State(state): State<AppState>, session: CurrentSession) -> Response {
    let body = SessionResponse {
        user: session.user.into(),
    };
    (
        [(
            header::SET_COOKIE,
            session.token.cookie(state.clients.cookie_secure),
        )],
        Json(body),
    )
        .into_response()
}

fn get_current_session_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag("Sessions")
        .summary("Get the current session")
        .description(
            "The session the cookie names, and its account. The session's 30 days start \
             again, and `Set-Cookie` renews the cookie to match.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<SessionResponse>, _>(|r| r.description("The session"))
        .response_with::<401, ApiError, _>(|r| {
            r.description("No cookie, or its session has expired or been closed")
        })
}

async fn delete_current_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    if let Some(token) = SessionToken::from_headers(&headers) {
        sessions::close(&state.pool, &token).await?;
    }
    Ok((
        [(
            header::SET_COOKIE,
            sessions::removal_cookie(state.clients.cookie_secure),
        )],
        NoContent,
    )
        .into_response())
}

fn delete_current_session_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag("Sessions")
        .summary("Log out")
        .description(
            "Closes the session the cookie names and clears the cookie. Answers 204 even \
             when there is no session, so that a retry is harmless.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<204, NoContent, _>(|r| r.description("Logged out"))
}
