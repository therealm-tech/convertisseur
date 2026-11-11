//! What learners study: the two tracks with their units and lessons, and a
//! lesson with its cards. Open to every logged-in account, and only ever
//! showing content validated together with every row above it.

use aide::{
    axum::{ApiRouter, routing::get_with},
    transform::TransformOperation,
};
use axum::{Json, extract::State};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    SESSION_SECURITY,
    content::{LetterFormsResponse, TrackJson, VarietyJson},
    extract::{ApiPath, Authenticated},
};
use crate::{
    content::Track,
    db,
    error::ApiError,
    router::AppState,
    tracks::{self, Outline, OutlineLesson, OutlineUnit, StudyCard},
};

pub const TAG: &str = "Tracks";

/// Both tracks, in the order the app presents them.
const TRACKS: [Track; 2] = [Track::Arabe, Track::Islam];

pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route("/api/v1/tracks", get_with(list_tracks, list_tracks_docs))
        .api_route(
            "/api/v1/tracks/{track}",
            get_with(get_track, get_track_docs),
        )
        .api_route(
            "/api/v1/tracks/{track}/lessons/{lessonId}",
            get_with(get_lesson, get_lesson_docs),
        )
}

/// Names one track in the path.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrackPath {
    track: TrackJson,
}

/// Names one lesson of a track in the path.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrackLessonPath {
    track: TrackJson,
    lesson_id: Uuid,
}

/// A lesson in a unit's outline.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrackLessonResponse {
    id: Uuid,
    title: String,
    /// How many cards the lesson shows.
    card_count: u64,
}

impl From<OutlineLesson> for TrackLessonResponse {
    fn from(lesson: OutlineLesson) -> Self {
        Self {
            id: lesson.id,
            title: lesson.title,
            card_count: u64::try_from(lesson.card_count).unwrap_or_default(),
        }
    }
}

/// A unit of a track, with its lessons in order.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrackUnitResponse {
    id: Uuid,
    title: String,
    lessons: Vec<TrackLessonResponse>,
}

impl From<OutlineUnit> for TrackUnitResponse {
    fn from(unit: OutlineUnit) -> Self {
        Self {
            id: unit.id,
            title: unit.title,
            lessons: unit.lessons.into_iter().map(Into::into).collect(),
        }
    }
}

/// A track, with its units in order; empty until something in it is
/// validated.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "id": "arabe",
    "units": [{
        "id": "0b8e6a8c-4d2f-4f0a-9a57-3d1c8f1e2b6d",
        "title": "L'alphabet",
        "lessons": [{
            "id": "5d0c1f8e-7a2b-4c3d-9e4f-1a2b3c4d5e6f",
            "title": "Les lettres, 1re partie",
            "cardCount": 7,
        }],
    }],
})))]
pub struct TrackResponse {
    id: TrackJson,
    units: Vec<TrackUnitResponse>,
}

impl From<Outline> for TrackResponse {
    fn from(outline: Outline) -> Self {
        Self {
            id: outline.track.into(),
            units: outline.units.into_iter().map(Into::into).collect(),
        }
    }
}

/// Both tracks, `arabe` then `islam`.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrackListResponse {
    items: Vec<TrackResponse>,
}

async fn list_tracks(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
) -> Result<Json<TrackListResponse>, ApiError> {
    let rows = db::tracks::outline(&state.pool, None).await?;
    tracing::debug!(user_id = %user.id, rows = rows.len(), "tracks listed");
    Ok(Json(TrackListResponse {
        items: tracks::outlines(&TRACKS, rows)
            .into_iter()
            .map(Into::into)
            .collect(),
    }))
}

fn list_tracks_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("List the tracks")
        .description(
            "Both tracks, each with its units and their lessons, in order. Only what is \
             validated together with every unit or lesson above it is listed. The list always \
             holds the two tracks, so it is not paginated.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<TrackListResponse>, _>(|r| r.description("Both tracks"))
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`password-change-required`: it must change its password first")
        })
}

async fn get_track(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    ApiPath(path): ApiPath<TrackPath>,
) -> Result<Json<TrackResponse>, ApiError> {
    let track = Track::from(path.track);
    let rows = db::tracks::outline(&state.pool, Some(track)).await?;
    tracing::debug!(user_id = %user.id, ?track, rows = rows.len(), "track read");
    let outline = tracks::outlines(&[track], rows)
        .pop()
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("no outline for {track:?}")))?;
    Ok(Json(outline.into()))
}

fn get_track_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Get a track")
        .description(
            "The track's units and their lessons, in order, as `GET /api/v1/tracks` lists them.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<TrackResponse>, _>(|r| r.description("The track"))
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`password-change-required`: it must change its password first")
        })
        .response_with::<404, ApiError, _>(|r| r.description("No track has this name"))
}

/// A recording to play.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackResponse {
    /// Where an `<audio>` element plays it from.
    url: String,
    /// `null` when the server does not know it.
    duration_ms: Option<u32>,
}

fn playback(audio_id: Option<Uuid>, duration_ms: Option<i32>) -> Option<PlaybackResponse> {
    audio_id.map(|id| PlaybackResponse {
        url: format!("/api/v1/audio/{id}"),
        duration_ms: duration_ms.and_then(|ms| u32::try_from(ms).ok()),
    })
}

/// A card to study.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StudyCardResponse {
    id: Uuid,
    /// Vocalised Arabic.
    arabic: String,
    transliteration: Option<String>,
    meaning_fr: Option<String>,
    variety: VarietyJson,
    /// The other shapes of the letter, for a card of the alphabet; `null`
    /// for any other card.
    forms: Option<LetterFormsResponse>,
    /// `null` when the card has no recording.
    audio: Option<PlaybackResponse>,
}

impl From<StudyCard> for StudyCardResponse {
    fn from(card: StudyCard) -> Self {
        Self {
            id: card.id,
            forms: LetterFormsResponse::of(&card.arabic, card.forms),
            arabic: card.arabic,
            transliteration: card.transliteration,
            meaning_fr: card.meaning_fr,
            variety: card.variety.into(),
            audio: playback(card.audio_id, card.audio_duration_ms),
        }
    }
}

/// The unit a lesson belongs to.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StudyUnitResponse {
    id: Uuid,
    title: String,
}

/// A lesson to study, with its cards in order.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "id": "5d0c1f8e-7a2b-4c3d-9e4f-1a2b3c4d5e6f",
    "track": "arabe",
    "unit": { "id": "0b8e6a8c-4d2f-4f0a-9a57-3d1c8f1e2b6d", "title": "Salutations" },
    "title": "En darija",
    "audio": null,
    "cards": [{
        "id": "9f8e7d6c-5b4a-4321-8fed-cba987654321",
        "arabic": "وَاشْ رَاكْ؟",
        "transliteration": "wach rak ?",
        "meaningFr": "ça va ?",
        "variety": "darija",
        "forms": null,
        "audio": { "url": "/api/v1/audio/3c2b1a09-8f7e-4d6c-b5a4-938271605f4e", "durationMs": 1200 },
    }],
})))]
pub struct StudyLessonResponse {
    id: Uuid,
    track: TrackJson,
    unit: StudyUnitResponse,
    title: String,
    /// `null` when the lesson has no recording.
    audio: Option<PlaybackResponse>,
    cards: Vec<StudyCardResponse>,
}

async fn get_lesson(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    ApiPath(path): ApiPath<TrackLessonPath>,
) -> Result<Json<StudyLessonResponse>, ApiError> {
    let lesson = db::tracks::find_lesson(&state.pool, path.lesson_id, path.track.into())
        .await?
        .ok_or_else(|| {
            tracing::debug!(user_id = %user.id, lesson_id = %path.lesson_id, "lesson not visible");
            ApiError::NotFound
        })?;
    let cards = db::tracks::cards(&state.pool, lesson.id).await?;
    tracing::debug!(user_id = %user.id, lesson_id = %lesson.id, cards = cards.len(), "lesson read");
    Ok(Json(StudyLessonResponse {
        id: lesson.id,
        track: lesson.track.into(),
        unit: StudyUnitResponse {
            id: lesson.unit_id,
            title: lesson.unit_title,
        },
        title: lesson.title,
        audio: playback(lesson.audio_id, lesson.audio_duration_ms),
        cards: cards.into_iter().map(Into::into).collect(),
    }))
}

fn get_lesson_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Get a lesson to study")
        .description(
            "The lesson, its recording, and its cards in order. The lesson is found only when \
             it and its unit are validated and the unit is in this track; only validated cards \
             are listed.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<StudyLessonResponse>, _>(|r| r.description("The lesson"))
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`password-change-required`: it must change its password first")
        })
        .response_with::<404, ApiError, _>(|r| {
            r.description("No lesson of this track that learners see has this id")
        })
}
