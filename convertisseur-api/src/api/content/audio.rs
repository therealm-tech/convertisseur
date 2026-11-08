//! Recordings: uploading one for a card or a lesson, removing it, and
//! playing it back.

use std::ops::RangeInclusive;

use aide::{
    axum::{ApiRouter, routing::get_with},
    transform::TransformOperation,
};
use axum::{
    Json,
    body::Body,
    extract::{DefaultBodyLimit, State, multipart::MultipartError},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, NoContent, Response},
};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;
use validator::{ValidationError, ValidationErrors};

use super::{AudioResponse, Resource, TAG};
use crate::{
    api::{
        SESSION_SECURITY,
        extract::{Admin, ApiMultipart, ApiPath, Authenticated, RequireRole},
    },
    audio::{self, Format},
    content::{self, Status, Upload},
    db::{self, content::WithAudio},
    error::ApiError,
    router::AppState,
    storage::ObjectStore,
    users::Role,
};

/// Room for the multipart boundaries and headers around the file.
const MULTIPART_OVERHEAD: usize = 64 * 1024;

/// The body limit of an upload route.
pub fn upload_limit() -> DefaultBodyLimit {
    DefaultBodyLimit::max(audio::MAX_BYTES + MULTIPART_OVERHEAD)
}

pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new().api_route("/api/v1/audio/{audioId}", get_with(play, play_docs))
}

fn invalid(code: &'static str) -> ApiError {
    let mut errors = ValidationErrors::new();
    errors.add("file", ValidationError::new(code));
    ApiError::Validation(errors)
}

fn multipart_failure(error: &MultipartError) -> ApiError {
    tracing::debug!(%error, "multipart body unreadable");
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::TooLarge
    } else {
        ApiError::MalformedBody
    }
}

/// The `file` part of the body, recognised from its first bytes.
async fn read_upload(ApiMultipart(mut multipart): ApiMultipart) -> Result<Upload, ApiError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| multipart_failure(&e))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let claimed = field.content_type().map(str::to_owned);
        let bytes = field.bytes().await.map_err(|e| multipart_failure(&e))?;
        if bytes.len() > audio::MAX_BYTES {
            return Err(ApiError::TooLarge);
        }
        if bytes.is_empty() {
            return Err(invalid("file_empty"));
        }
        let Some(format) = Format::sniff(&bytes) else {
            tracing::debug!(?claimed, size = bytes.len(), "upload is no known audio");
            return Err(ApiError::UnsupportedAudio);
        };
        tracing::debug!(?claimed, ?format, size = bytes.len(), "upload recognised");
        return Ok(Upload { format, bytes });
    }
    Err(invalid("file_missing"))
}

pub async fn put_audio<R: Resource<Rows: WithAudio>>(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ApiPath(path): ApiPath<R::Path>,
    multipart: ApiMultipart,
) -> Result<Json<AudioResponse>, ApiError> {
    let upload = read_upload(multipart).await?;
    let audio =
        content::attach_audio::<R::Rows>(&state.pool, &state.storage, &actor, path.into(), upload)
            .await?;
    Ok(Json(audio.into()))
}

pub fn put_audio_docs<R: Resource>(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Upload a recording")
        .description(
            "A `multipart/form-data` body whose `file` part is an MP3, MP4 audio (AAC, `.m4a`), \
             Ogg (Opus or Vorbis), WebM (Opus) or WAV file of at most 10 MB. The type is read \
             from the file's first bytes, whatever the client says, and the file is stored \
             as is. It replaces the recording the item had, which is deleted, and sends the \
             item back to `to_review`.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<AudioResponse>, _>(|r| r.description("The new recording"))
        .response_with::<400, ApiError, _>(|r| r.description("The body is not valid multipart"))
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<404, ApiError, _>(|r| r.description(R::NOT_FOUND_DOCS))
        .response_with::<413, ApiError, _>(|r| r.description("The file is larger than 10 MB"))
        .response_with::<415, ApiError, _>(|r| {
            r.description(
                "`unsupported-media-type`: the body is not `multipart/form-data`; \
                 `unsupported-audio`: the file is none of the accepted formats",
            )
        })
        .response_with::<422, ApiError, _>(|r| {
            r.description("`file_missing`: no `file` part; `file_empty`: it is empty")
        })
}

pub async fn delete_audio<R: Resource<Rows: WithAudio>>(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ApiPath(path): ApiPath<R::Path>,
) -> Result<NoContent, ApiError> {
    content::detach_audio::<R::Rows>(&state.pool, &state.storage, &actor, path.into()).await?;
    Ok(NoContent)
}

pub fn delete_audio_docs<R: Resource>(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Remove the recording")
        .description(
            "Deletes the recording, and sends the item back to `to_review`. Answers 204 when \
             it has none.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<204, NoContent, _>(|r| r.description("The item has no recording"))
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<404, ApiError, _>(|r| r.description(R::NOT_FOUND_DOCS))
}

/// Names one recording in the path.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AudioPath {
    audio_id: Uuid,
}

/// What a `Range` header asks of a file of `size` bytes.
#[derive(Debug, PartialEq, Eq)]
enum Ranged {
    Whole,
    Part(RangeInclusive<u64>),
    Unsatisfiable,
}

/// Reads a single `bytes` range. A header that does not parse, or asks for
/// several ranges, is ignored and the whole file served, as RFC 9110 allows.
fn requested_range(headers: &HeaderMap, size: u64) -> Ranged {
    let Some(spec) = headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().strip_prefix("bytes="))
    else {
        return Ranged::Whole;
    };
    if spec.contains(',') {
        return Ranged::Whole;
    }
    let Some((first, last)) = spec.split_once('-') else {
        return Ranged::Whole;
    };
    let (first, last) = (first.trim(), last.trim());
    if first.is_empty() {
        return match last.parse::<u64>() {
            Ok(0) => Ranged::Unsatisfiable,
            Ok(suffix) => Ranged::Part(size.saturating_sub(suffix)..=size - 1),
            Err(_) => Ranged::Whole,
        };
    }
    let Ok(first) = first.parse::<u64>() else {
        return Ranged::Whole;
    };
    let last = if last.is_empty() {
        size - 1
    } else {
        match last.parse::<u64>() {
            Ok(last) if last >= first => last.min(size - 1),
            _ => return Ranged::Whole,
        }
    };
    if first >= size {
        Ranged::Unsatisfiable
    } else {
        Ranged::Part(first..=last)
    }
}

async fn play(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    ApiPath(path): ApiPath<AudioPath>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let playable = db::audio::find_playable(&state.pool, path.audio_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if user.role < Role::Admin && playable.status != Some(Status::Validated) {
        tracing::debug!(user_id = %user.id, audio_id = %path.audio_id, "recording not validated");
        return Err(ApiError::NotFound);
    }
    let size = u64::try_from(playable.size_bytes).unwrap_or_default();
    let (status, range) = match requested_range(&headers, size) {
        Ranged::Whole => (StatusCode::OK, None),
        Ranged::Part(range) => (StatusCode::PARTIAL_CONTENT, Some(range)),
        Ranged::Unsatisfiable => return Err(ApiError::RangeNotSatisfiable { size }),
    };
    let body = state
        .storage
        .get(&playable.object_key, range.clone())
        .await?
        .ok_or_else(|| {
            ApiError::Internal(anyhow::anyhow!(
                "object {} is missing from the store",
                playable.object_key
            ))
        })?;
    tracing::debug!(audio_id = %path.audio_id, ?range, "recording played");

    let mut response = (status, Body::new(body)).into_response();
    let headers = response.headers_mut();
    let header = |value: String| HeaderValue::try_from(value).map_err(anyhow::Error::from);
    headers.insert(header::CONTENT_TYPE, header(playable.content_type)?);
    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("private"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    let length = match &range {
        Some(range) => {
            headers.insert(
                header::CONTENT_RANGE,
                header(format!("bytes {}-{}/{size}", range.start(), range.end()))?,
            );
            range.end() - range.start() + 1
        }
        None => size,
    };
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(length));
    Ok(response)
}

fn play_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Play a recording")
        .description(
            "Streams the file with its own `Content-Type`. Honours a single `Range: bytes=…`, \
             which `<audio>` seeking needs, with a 206. Any logged-in account plays the \
             recordings of validated content; an `admin` plays them all. Others answer 404.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, (), _>(|r| r.description("The whole file, `audio/*`"))
        .response_with::<206, (), _>(|r| r.description("The requested range"))
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`password-change-required`: it must change its password first")
        })
        .response_with::<404, ApiError, _>(|r| {
            r.description("No recording has this id, or it is not for this account")
        })
        .response_with::<416, ApiError, _>(|r| {
            r.description("The range starts past the end; `Content-Range` gives the size")
        })
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, header};

    use super::{Ranged, requested_range};

    fn range(value: &str) -> Ranged {
        let mut headers = HeaderMap::new();
        headers.insert(header::RANGE, HeaderValue::from_str(value).unwrap());
        requested_range(&headers, 1000)
    }

    #[test]
    fn reads_single_byte_ranges() {
        assert_eq!(requested_range(&HeaderMap::new(), 1000), Ranged::Whole);
        assert_eq!(range("bytes=0-99"), Ranged::Part(0..=99));
        assert_eq!(range("bytes=500-"), Ranged::Part(500..=999));
        assert_eq!(range("bytes=900-5000"), Ranged::Part(900..=999));
        assert_eq!(range("bytes=-100"), Ranged::Part(900..=999));
        assert_eq!(range("bytes=-5000"), Ranged::Part(0..=999));
        assert_eq!(range("bytes=0-"), Ranged::Part(0..=999));
    }

    #[test]
    fn refuses_ranges_past_the_end() {
        assert_eq!(range("bytes=1000-"), Ranged::Unsatisfiable);
        assert_eq!(range("bytes=1000-1001"), Ranged::Unsatisfiable);
        assert_eq!(range("bytes=-0"), Ranged::Unsatisfiable);
    }

    #[test]
    fn serves_the_whole_file_for_what_it_does_not_read() {
        for value in [
            "items=0-10",
            "bytes=0-10,20-30",
            "bytes=10-5",
            "bytes=x-",
            "bytes=-x",
            "bytes",
        ] {
            assert_eq!(range(value), Ranged::Whole, "{value}");
        }
    }
}
