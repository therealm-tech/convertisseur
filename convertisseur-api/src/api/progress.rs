//! What a learner records as it studies: the cards it sees and its exercise
//! attempts. Each write is about the session's own account.

use aide::{
    axum::{
        ApiRouter,
        routing::{post_with, put_with},
    },
    transform::TransformOperation,
};
use axum::{Json, extract::State, http::StatusCode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError, ValidationErrors};

use super::{
    SESSION_SECURITY,
    extract::{ApiPath, Authenticated, ValidJson},
    tracks::ExerciseResultResponse,
};
use crate::{db, error::ApiError, exercise, progress, router::AppState};

pub const TAG: &str = "Progress";

/// More questions than an exercise ever asks.
const MAX_QUESTIONS: u32 = 1000;

pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route(
            "/api/v1/card-views/{cardId}",
            put_with(record_view, record_view_docs),
        )
        .api_route(
            "/api/v1/exercise-attempts",
            post_with(record_attempt, record_attempt_docs),
        )
}

/// Names one card in the path.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CardPath {
    card_id: Uuid,
}

async fn record_view(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    ApiPath(path): ApiPath<CardPath>,
) -> Result<StatusCode, ApiError> {
    let (lesson_id, track) = db::progress::find_card(&state.pool, path.card_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let progress = progress::load(&state.pool, user.id, &[track]).await?;
    if progress.lessons.get(&lesson_id).is_some_and(|s| s.locked) {
        tracing::debug!(user_id = %user.id, %lesson_id, "card view refused, lesson locked");
        return Err(ApiError::LessonLocked);
    }
    db::progress::record_view(&state.pool, user.id, path.card_id).await?;
    tracing::debug!(user_id = %user.id, card_id = %path.card_id, "card view recorded");
    Ok(StatusCode::NO_CONTENT)
}

fn record_view_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Record that the account saw a card")
        .description(
            "Marks the card as seen by the session's account, keeping when it first and last \
             saw it. Sending it again only moves the last time, so a retry is harmless.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<204, (), _>(|r| r.description("Recorded"))
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description(
                "`password-change-required`: it must change its password first; \
                 `lesson-locked`: the card's lesson is locked for the account",
            )
        })
        .response_with::<404, ApiError, _>(|r| r.description("No card learners see has this id"))
}

/// One run through a lesson's exercise.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "lessonId": "5d0c1f8e-7a2b-4c3d-9e4f-1a2b3c4d5e6f",
    "firstTry": 11,
    "total": 12,
})))]
pub struct AttemptRequest {
    lesson_id: Uuid,
    /// Questions answered right the first time, at most `total`.
    first_try: u32,
    /// Questions asked, 1 to 1000.
    #[validate(range(min = 1, max = MAX_QUESTIONS))]
    total: u32,
}

/// A lesson the account may open.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NextLessonResponse {
    id: Uuid,
    title: String,
}

/// The account's result in the lesson once the attempt is counted.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttemptResponse {
    exercise: ExerciseResultResponse,
    /// The lesson after this one on the track, when the account may open
    /// it; `null` at the end of the track or while it stays locked.
    next_lesson: Option<NextLessonResponse>,
}

async fn record_attempt(
    State(state): State<AppState>,
    Authenticated(user): Authenticated,
    ValidJson(attempt): ValidJson<AttemptRequest>,
) -> Result<Json<AttemptResponse>, ApiError> {
    if attempt.first_try > attempt.total {
        return Err(invalid("first_try", "first_try_above_total"));
    }
    let track = db::progress::find_lesson(&state.pool, attempt.lesson_id)
        .await?
        .ok_or_else(|| invalid("lesson_id", "lesson_not_found"))?;
    let before = progress::load(&state.pool, user.id, &[track]).await?;
    if before
        .lessons
        .get(&attempt.lesson_id)
        .is_some_and(|s| s.locked)
    {
        tracing::debug!(user_id = %user.id, lesson_id = %attempt.lesson_id, "attempt refused, lesson locked");
        return Err(ApiError::LessonLocked);
    }
    let first_try = i32::try_from(attempt.first_try).map_err(anyhow::Error::from)?;
    let total = i32::try_from(attempt.total).map_err(anyhow::Error::from)?;
    let passed = exercise::passes(i64::from(first_try), i64::from(total));
    let result = db::progress::record_attempt(
        &state.pool,
        user.id,
        attempt.lesson_id,
        first_try,
        total,
        passed,
    )
    .await?;
    tracing::debug!(
        user_id = %user.id,
        lesson_id = %attempt.lesson_id,
        first_try,
        total,
        passed,
        "exercise attempt recorded"
    );
    let after = progress::load(&state.pool, user.id, &[track]).await?;
    let next_lesson = after
        .next_lesson(attempt.lesson_id)
        .filter(|next| after.lessons.get(&next.id).is_some_and(|s| !s.locked))
        .map(|next| NextLessonResponse {
            id: next.id,
            title: next.title.clone(),
        });
    Ok(Json(AttemptResponse {
        exercise: result.into(),
        next_lesson,
    }))
}

fn invalid(field: &'static str, code: &'static str) -> ApiError {
    let mut errors = ValidationErrors::new();
    errors.add(field, ValidationError::new(code));
    ApiError::Validation(errors)
}

fn record_attempt_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Count an exercise attempt")
        .description(
            "Counts a run through the lesson's exercise for the session's account. The server \
             keeps the best share of questions answered right the first time, not every \
             attempt, so it answers the result rather than a new resource: `200`, not `201`. \
             90% passes the exercise and opens the next lesson; a pass is never taken back. \
             The exercise runs in the webapp, so the server takes the score as sent.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<AttemptResponse>, _>(|r| r.description("The result"))
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description(
                "`password-change-required`: it must change its password first; \
                 `lesson-locked`: the lesson is locked for the account",
            )
        })
        .response_with::<422, ApiError, _>(|r| {
            r.description(
                "`total` is not 1 to 1000, `first_try_above_total`, or `lesson_not_found`: no \
                 lesson learners see has this id",
            )
        })
}
