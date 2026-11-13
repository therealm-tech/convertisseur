//! What learners study: the two tracks with their units and lessons, and a
//! lesson with its cards, each with where the account stands in it. Open to
//! every logged-in account, and only ever showing content validated together
//! with every row above it.

use aide::{
    axum::{ApiRouter, routing::get_with},
    transform::TransformOperation,
};
use std::collections::{HashMap, HashSet};

use axum::{Json, extract::State};
use chrono::{DateTime, Utc};
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
    progress::{self, ExerciseResult, LessonState},
    router::AppState,
    tracks::{Outline, OutlineLesson, OutlineUnit, StudyCard},
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

/// A score: questions answered right the first time, of those asked.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScoreResponse {
    first_try: u32,
    total: u32,
    at: DateTime<Utc>,
}

/// The account's exercise result in a lesson.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExerciseResultResponse {
    /// The best share of questions answered right the first time; a tie
    /// keeps the earlier score.
    best: ScoreResponse,
    /// When the account first passed the exercise; `null` until it has.
    /// A pass is never taken back.
    passed_at: Option<DateTime<Utc>>,
    last_attempt_at: DateTime<Utc>,
}

impl From<ExerciseResult> for ExerciseResultResponse {
    fn from(result: ExerciseResult) -> Self {
        Self {
            best: ScoreResponse {
                first_try: u32::try_from(result.best_first_try).unwrap_or_default(),
                total: u32::try_from(result.best_total).unwrap_or_default(),
                at: result.best_at,
            },
            passed_at: result.passed_at,
            last_attempt_at: result.last_attempt_at,
        }
    }
}

/// Where the account stands in a lesson.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LessonProgressResponse {
    /// How many of the cards the lesson shows the account has seen.
    seen_card_count: u64,
    /// When it last saw one of the lesson's cards; `null` if never.
    last_seen_at: Option<DateTime<Utc>>,
    /// `null` until the account sends an exercise result.
    exercise: Option<ExerciseResultResponse>,
}

/// A lesson in a unit's outline.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrackLessonResponse {
    id: Uuid,
    title: String,
    /// How many cards the lesson shows.
    card_count: u64,
    /// Whether its cards make an exercise.
    has_exercise: bool,
    /// Whether the account may not open it yet: the lesson before it is
    /// locked, or has an exercise the account has not passed. A lesson the
    /// account has started is never locked.
    locked: bool,
    progress: LessonProgressResponse,
}

impl TrackLessonResponse {
    fn new(lesson: OutlineLesson, state: Option<LessonState>) -> Self {
        let state = state.unwrap_or(LessonState {
            has_exercise: false,
            locked: false,
            seen_card_count: 0,
            last_seen_at: None,
            exercise: None,
        });
        Self {
            id: lesson.id,
            title: lesson.title,
            card_count: u64::try_from(lesson.card_count).unwrap_or_default(),
            has_exercise: state.has_exercise,
            locked: state.locked,
            progress: LessonProgressResponse {
                seen_card_count: u64::try_from(state.seen_card_count).unwrap_or_default(),
                last_seen_at: state.last_seen_at,
                exercise: state.exercise.map(Into::into),
            },
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

impl TrackUnitResponse {
    fn new(unit: OutlineUnit, states: &mut HashMap<Uuid, LessonState>) -> Self {
        Self {
            id: unit.id,
            title: unit.title,
            lessons: unit
                .lessons
                .into_iter()
                .map(|lesson| {
                    let state = states.remove(&lesson.id);
                    TrackLessonResponse::new(lesson, state)
                })
                .collect(),
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
            "hasExercise": true,
            "locked": false,
            "progress": {
                "seenCardCount": 7,
                "lastSeenAt": "2026-11-10T18:04:12Z",
                "exercise": {
                    "best": { "firstTry": 11, "total": 12, "at": "2026-11-10T18:09:40Z" },
                    "passedAt": "2026-11-10T18:09:40Z",
                    "lastAttemptAt": "2026-11-10T18:09:40Z",
                },
            },
        }],
    }],
})))]
pub struct TrackResponse {
    id: TrackJson,
    units: Vec<TrackUnitResponse>,
}

impl TrackResponse {
    fn new(outline: Outline, states: &mut HashMap<Uuid, LessonState>) -> Self {
        Self {
            id: outline.track.into(),
            units: outline
                .units
                .into_iter()
                .map(|unit| TrackUnitResponse::new(unit, states))
                .collect(),
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
    let mut progress = progress::load(&state.pool, user.id, &TRACKS).await?;
    tracing::debug!(user_id = %user.id, lessons = progress.lessons.len(), "tracks listed");
    Ok(Json(TrackListResponse {
        items: progress
            .outlines
            .into_iter()
            .map(|outline| TrackResponse::new(outline, &mut progress.lessons))
            .collect(),
    }))
}

fn list_tracks_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("List the tracks")
        .description(
            "Both tracks, each with its units and their lessons, in order, and where the \
             account stands in each lesson. Only what is validated together with every unit or \
             lesson above it is listed. The list always holds the two tracks, so it is not \
             paginated.",
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
    let mut progress = progress::load(&state.pool, user.id, &[track]).await?;
    tracing::debug!(user_id = %user.id, ?track, lessons = progress.lessons.len(), "track read");
    let outline = progress
        .outlines
        .pop()
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("no outline for {track:?}")))?;
    Ok(Json(TrackResponse::new(outline, &mut progress.lessons)))
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
    /// Whether the account has seen the card.
    seen: bool,
}

impl StudyCardResponse {
    fn new(card: StudyCard, seen: &HashSet<Uuid>) -> Self {
        Self {
            seen: seen.contains(&card.id),
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
        "seen": true,
    }],
    "exercise": null,
})))]
pub struct StudyLessonResponse {
    id: Uuid,
    track: TrackJson,
    unit: StudyUnitResponse,
    title: String,
    /// `null` when the lesson has no recording.
    audio: Option<PlaybackResponse>,
    cards: Vec<StudyCardResponse>,
    /// The account's exercise result; `null` until it sends one.
    exercise: Option<ExerciseResultResponse>,
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
    let progress = progress::load(&state.pool, user.id, &[lesson.track]).await?;
    let lesson_state = progress.lessons.get(&lesson.id);
    if lesson_state.is_some_and(|s| s.locked) {
        tracing::debug!(user_id = %user.id, lesson_id = %lesson.id, "lesson locked");
        return Err(ApiError::LessonLocked);
    }
    let exercise = lesson_state.and_then(|s| s.exercise.clone());
    let cards = db::tracks::cards(&state.pool, lesson.id).await?;
    let seen: HashSet<Uuid> = db::progress::seen_cards(&state.pool, user.id, lesson.id)
        .await?
        .into_iter()
        .collect();
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
        cards: cards
            .into_iter()
            .map(|card| StudyCardResponse::new(card, &seen))
            .collect(),
        exercise: exercise.map(Into::into),
    }))
}

fn get_lesson_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Get a lesson to study")
        .description(
            "The lesson, its recording, its cards in order, which of them the account has seen, \
             and its exercise result. The lesson is found only when it and its unit are \
             validated and the unit is in this track; only validated cards are listed.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<StudyLessonResponse>, _>(|r| r.description("The lesson"))
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description(
                "`password-change-required`: it must change its password first; \
                 `lesson-locked`: the account has not passed the exercise of the lesson before",
            )
        })
        .response_with::<404, ApiError, _>(|r| {
            r.description("No lesson of this track that learners see has this id")
        })
}
