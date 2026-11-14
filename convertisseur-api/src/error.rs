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
use validator::{ValidationError, ValidationErrors};

use crate::accounts::Refusal;

const PROBLEM_JSON: &str = "application/problem+json";

/// Every failure of the JSON API, answered as an RFC 9457 problem document.
#[derive(Debug)]
pub enum ApiError {
    NotFound,
    MalformedBody,
    MalformedQuery,
    UnsupportedMediaType,
    MultipartRequired,
    TooLarge,
    UnsupportedAudio,
    RangeNotSatisfiable { size: u64 },
    Validation(ValidationErrors),
    Unauthenticated,
    PasswordChangeRequired,
    Forbidden,
    LessonLocked,
    InvalidCredentials,
    UsernameTaken,
    EmailTaken,
    NotEmpty,
    Imported,
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

impl From<Refusal> for ApiError {
    fn from(refusal: Refusal) -> Self {
        let invalid = |field, code| {
            let mut errors = ValidationErrors::new();
            errors.add(field, ValidationError::new(code));
            Self::Validation(errors)
        };
        match refusal {
            Refusal::NotFound => Self::NotFound,
            Refusal::Forbidden => Self::Forbidden,
            Refusal::WrongPassword => invalid("current_password", "password_incorrect"),
            Refusal::SamePassword => invalid("new_password", "password_unchanged"),
            Refusal::Failed(error) => Self::Internal(error),
        }
    }
}

impl ApiError {
    // One arm per kind of problem: the catalogue reads best in one place.
    #[allow(clippy::too_many_lines)]
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
            Self::MalformedQuery => (
                StatusCode::BAD_REQUEST,
                "urn:convertisseur:problem:malformed-query",
                "Malformed query",
                "a query parameter is not of the expected type",
            ),
            Self::UnsupportedMediaType => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "urn:convertisseur:problem:unsupported-media-type",
                "Unsupported media type",
                "the body must be sent as application/json",
            ),
            Self::MultipartRequired => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "urn:convertisseur:problem:unsupported-media-type",
                "Unsupported media type",
                "the body must be sent as multipart/form-data",
            ),
            Self::TooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "urn:convertisseur:problem:too-large",
                "Too large",
                "the file is larger than 10 MB",
            ),
            Self::UnsupportedAudio => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "urn:convertisseur:problem:unsupported-audio",
                "Unsupported audio",
                "the file is not MP3, MP4 audio, Ogg, WebM or WAV",
            ),
            Self::RangeNotSatisfiable { .. } => (
                StatusCode::RANGE_NOT_SATISFIABLE,
                "urn:convertisseur:problem:range-not-satisfiable",
                "Range not satisfiable",
                "the requested range starts past the end of the file",
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
            Self::PasswordChangeRequired => (
                StatusCode::FORBIDDEN,
                "urn:convertisseur:problem:password-change-required",
                "Password change required",
                "the account must choose a new password before anything else",
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "urn:convertisseur:problem:forbidden",
                "Forbidden",
                "the account may not do this",
            ),
            Self::LessonLocked => (
                StatusCode::FORBIDDEN,
                "urn:convertisseur:problem:lesson-locked",
                "Lesson locked",
                "the exercise of the lesson before must be passed first",
            ),
            Self::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "urn:convertisseur:problem:invalid-credentials",
                "Invalid credentials",
                "no account has this username or email, or the password is wrong",
            ),
            Self::UsernameTaken => (
                StatusCode::CONFLICT,
                "urn:convertisseur:problem:username-taken",
                "Username taken",
                "an account already has this username, whatever its case",
            ),
            Self::EmailTaken => (
                StatusCode::CONFLICT,
                "urn:convertisseur:problem:email-taken",
                "Email taken",
                "an account already has this email, whatever its case",
            ),
            Self::NotEmpty => (
                StatusCode::CONFLICT,
                "urn:convertisseur:problem:not-empty",
                "Not empty",
                "it still holds lessons or cards; delete or move them first",
            ),
            Self::Imported => (
                StatusCode::CONFLICT,
                "urn:convertisseur:problem:imported-content",
                "Imported content",
                "it comes from the content files, where its text and place are changed; its \
                 status, recording and review note stay editable here",
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
        let mut content_range = None;
        match self {
            Self::Internal(error) => {
                tracing::error!(error = format!("{error:#}"), "request failed");
            }
            Self::Validation(invalid) => {
                for (field, field_errors) in invalid.field_errors() {
                    errors.extend(field_errors.iter().map(|e| FieldProblem {
                        field: camel_case(&field),
                        code: e.code.to_string(),
                    }));
                }
            }
            Self::RateLimited { retry_after: wait } => {
                retry_after = Some(wait.as_secs() + u64::from(wait.subsec_nanos() > 0));
            }
            Self::RangeNotSatisfiable { size } => {
                content_range = HeaderValue::try_from(format!("bytes */{size}")).ok();
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
        if let Some(range) = content_range {
            headers.insert(header::CONTENT_RANGE, range);
        }
        response
    }
}

/// validator names a field as Rust does; the wire names it in camelCase.
fn camel_case(field: &str) -> String {
    let mut words = field.split('_');
    let mut name = words.next().unwrap_or_default().to_owned();
    for word in words {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            name.extend(first.to_uppercase());
            name.push_str(chars.as_str());
        }
    }
    name
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

#[cfg(test)]
mod tests {
    #[test]
    fn names_fields_in_camel_case() {
        assert_eq!(super::camel_case("username"), "username");
        assert_eq!(super::camel_case("current_password"), "currentPassword");
        assert_eq!(super::camel_case("per_page"), "perPage");
    }
}
