use axum::{
    Json,
    http::{HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// Every failure of the JSON API, answered as an RFC 9457 problem document.
#[derive(Debug)]
pub enum ApiError {
    NotFound,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProblemResponse {
    /// Stable, machine-readable identifier of the problem kind.
    #[serde(rename = "type")]
    kind: &'static str,
    title: &'static str,
    status: u16,
    detail: &'static str,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, kind, title, detail) = match self {
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "urn:convertisseur:problem:not-found",
                "Not found",
                "the resource does not exist",
            ),
        };
        let body = ProblemResponse {
            kind,
            title,
            status: status.as_u16(),
            detail,
        };
        let mut response = (status, Json(body)).into_response();
        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        response
    }
}
