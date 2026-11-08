use std::time::Duration;

use aide::{
    OperationOutput,
    generate::GenContext,
    openapi::{Operation, Response as ApiResponse},
};
use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use schemars::JsonSchema;
use serde::Serialize;
use validator::ValidationErrors;

const PROBLEM_JSON: &str = "application/problem+json";

/// Every failure of the JSON API, answered as an RFC 9457 problem document.
#[derive(Debug)]
pub enum ApiError {
    NotFound,
    MalformedBody,
    UnsupportedMediaType,
    Validation(ValidationErrors),
    Unauthenticated,
    InvalidCredentials,
    UsernameTaken,
    RateLimited { retry_after: Duration },
    Internal(anyhow::Error),
}

/// An RFC 9457 problem document.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProblemResponse {
    /// Stable, machine-readable identifier of the problem kind, e.g.
    /// `urn:convertisseur:problem:validation`.
    #[serde(rename = "type")]
    kind: &'static str,
    title: &'static str,
    status: u16,
    detail: &'static str,
    /// The broken rules, on a validation problem.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<FieldProblem>,
}

/// One rule a request field breaks.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FieldProblem {
    /// The field, as named in the request body.
    field: String,
    /// Stable identifier of the rule, e.g. `username_length`.
    code: String,
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        Self::Internal(error.into())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

impl From<ValidationErrors> for ApiError {
    fn from(errors: ValidationErrors) -> Self {
        Self::Validation(errors)
    }
}

impl ApiError {
    fn describe(&self) -> (StatusCode, &'static str, &'static str, &'static str) {
        match self {
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "urn:convertisseur:problem:not-found",
                "Not found",
                "the resource does not exist",
            ),
            Self::MalformedBody => (
                StatusCode::BAD_REQUEST,
                "urn:convertisseur:problem:malformed-body",
                "Malformed body",
                "the body is not JSON of the expected shape",
            ),
            Self::UnsupportedMediaType => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "urn:convertisseur:problem:unsupported-media-type",
                "Unsupported media type",
                "the body must be sent as application/json",
            ),
            Self::Validation(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "urn:convertisseur:problem:validation",
                "Invalid request",
                "some fields break a rule, listed in errors",
            ),
            Self::Unauthenticated => (
                StatusCode::UNAUTHORIZED,
                "urn:convertisseur:problem:unauthenticated",
                "Not logged in",
                "the request carries no live session",
            ),
            Self::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "urn:convertisseur:problem:invalid-credentials",
                "Invalid credentials",
                "the username or the password is wrong",
            ),
            Self::UsernameTaken => (
                StatusCode::CONFLICT,
                "urn:convertisseur:problem:username-taken",
                "Username taken",
                "an account already has this username, whatever its case",
            ),
            Self::RateLimited { .. } => (
                StatusCode::TOO_MANY_REQUESTS,
                "urn:convertisseur:problem:rate-limited",
                "Too many requests",
                "too many attempts from this address; retry after Retry-After seconds",
            ),
            Self::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "urn:convertisseur:problem:internal",
                "Internal error",
                "the server failed to answer",
            ),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, kind, title, detail) = self.describe();
        let mut errors = Vec::new();
        let mut retry_after = None;
        match self {
            Self::Internal(error) => {
                tracing::error!(error = format!("{error:#}"), "request failed");
            }
            Self::Validation(invalid) => {
                for (field, field_errors) in invalid.field_errors() {
                    errors.extend(field_errors.iter().map(|e| FieldProblem {
                        field: field.to_string(),
                        code: e.code.to_string(),
                    }));
                }
            }
            Self::RateLimited { retry_after: wait } => {
                retry_after = Some(wait.as_secs() + u64::from(wait.subsec_nanos() > 0));
            }
            _ => {}
        }
        let body = ProblemResponse {
            kind,
            title,
            status: status.as_u16(),
            detail,
            errors,
        };
        let mut response = (status, Json(body)).into_response();
        let headers = response.headers_mut();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(PROBLEM_JSON));
        if let Some(seconds) = retry_after {
            headers.insert(header::RETRY_AFTER, HeaderValue::from(seconds));
        }
        response
    }
}

/// Documents an error as a problem document; each operation declares the
/// statuses it answers.
impl OperationOutput for ApiError {
    type Inner = ProblemResponse;

    fn operation_response(ctx: &mut GenContext, operation: &mut Operation) -> Option<ApiResponse> {
        let mut response = Json::<ProblemResponse>::operation_response(ctx, operation)?;
        if let Some(media) = response.content.shift_remove("application/json") {
            response.content.insert(PROBLEM_JSON.to_owned(), media);
        }
        Some(response)
    }
}
