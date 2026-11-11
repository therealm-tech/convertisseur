//! The review queue: lessons with the review and audio state of their
//! cards, a lesson's cards in order, and their validation in one go.

use aide::{
    axum::{
        ApiRouter,
        routing::{get_with, post_with},
    },
    transform::TransformOperation,
};
use axum::{Json, extract::State};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError, ValidationErrors};

use super::{
    AuthorResponse, DEFAULT_PER_PAGE, MAX_PER_PAGE, StatusJson, TAG, TrackJson, author,
    cards::CardResponse, lessons::LessonPath,
};
use crate::{
    api::{
        SESSION_SECURITY,
        extract::{Admin, ApiPath, RequireRole, ValidJson, ValidQuery},
    },
    content::{self, ContentRefusal},
    db,
    error::ApiError,
    review::{self, CardReview, LessonFilter, LessonSummary},
    router::AppState,
};

/// At most this many cards are validated in one request.
const MAX_CARDS_PER_VALIDATION: u64 = 100;

pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route(
            "/api/v1/lessons/{lessonId}/cards",
            get_with(list_lesson_cards, list_lesson_cards_docs),
        )
        .api_route(
            "/api/v1/lessons/{lessonId}/card-validations",
            post_with(validate_cards, validate_cards_docs),
        )
}

fn count(n: i64) -> u64 {
    u64::try_from(n).unwrap_or_default()
}

/// Which lessons to list, and which page of them.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ListLessonsQuery {
    /// 1-based; 1 by default.
    #[validate(range(min = 1))]
    page: Option<u32>,
    /// 1 to 100; 50 by default.
    #[validate(range(min = 1, max = MAX_PER_PAGE))]
    per_page: Option<u32>,
    /// Only the lessons of this unit.
    unit_id: Option<Uuid>,
    /// `true` lists only the lessons with something to review: the lesson,
    /// its unit or one of its cards.
    pending_review: Option<bool>,
    /// `true` lists only the lessons with a card without a recording,
    /// whatever their status.
    missing_audio: Option<bool>,
}

/// The unit a lesson of the queue belongs to.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueueUnitResponse {
    id: Uuid,
    track: TrackJson,
    /// 1-based place among the track's units.
    position: i32,
    title: String,
    status: StatusJson,
    review_note: Option<String>,
}

/// How a lesson's cards stand. `validated`, `flagged` and `toReview` add up
/// to the lesson's cards: a flagged card is to review with a review note.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CardCountsResponse {
    validated: u64,
    flagged: u64,
    /// The cards to review without a review note.
    to_review: u64,
    /// The cards without a recording, whatever their status.
    without_audio: u64,
}

/// A lesson of the queue.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LessonSummaryResponse {
    id: Uuid,
    /// 1-based place among the unit's lessons.
    position: i32,
    title: String,
    status: StatusJson,
    review_note: Option<String>,
    /// Whether the lesson holds a recording of its own, which is optional.
    has_audio: bool,
    unit: QueueUnitResponse,
    card_counts: CardCountsResponse,
}

impl From<LessonSummary> for LessonSummaryResponse {
    fn from(lesson: LessonSummary) -> Self {
        Self {
            id: lesson.id,
            position: lesson.position,
            title: lesson.title,
            status: lesson.status.into(),
            review_note: lesson.review_note,
            has_audio: lesson.has_audio,
            unit: QueueUnitResponse {
                id: lesson.unit_id,
                track: lesson.unit_track.into(),
                position: lesson.unit_position,
                title: lesson.unit_title,
                status: lesson.unit_status.into(),
                review_note: lesson.unit_review_note,
            },
            card_counts: CardCountsResponse {
                validated: count(lesson.validated_cards),
                flagged: count(lesson.flagged_cards),
                to_review: count(lesson.cards_to_review),
                without_audio: count(lesson.cards_without_audio),
            },
        }
    }
}

/// One page of lessons, in the order of the tracks: units by track and
/// position, lessons by position.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LessonSummaryPageResponse {
    items: Vec<LessonSummaryResponse>,
    page: u32,
    per_page: u32,
    /// How many lessons match the filters, across all pages.
    total: u64,
    /// Units, lessons and cards to review, flagged or not, whatever the
    /// filters.
    items_to_review: u64,
    /// Cards without a recording, whatever the filters.
    cards_without_audio: u64,
}

pub(super) async fn list_lessons(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ValidQuery(query): ValidQuery<ListLessonsQuery>,
) -> Result<Json<LessonSummaryPageResponse>, ApiError> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(DEFAULT_PER_PAGE);
    let filter = LessonFilter {
        unit_id: query.unit_id,
        pending_review: query.pending_review.unwrap_or(false),
        missing_audio: query.missing_audio.unwrap_or(false),
    };
    let offset = i64::from(page - 1) * i64::from(per_page);
    let lessons = db::review::lessons(&state.pool, &filter, per_page.into(), offset).await?;
    let total = count(db::review::count_lessons(&state.pool, &filter).await?);
    let totals = db::review::totals(&state.pool).await?;
    tracing::debug!(actor_id = %actor.id, page, per_page, total, "lessons listed");
    Ok(Json(LessonSummaryPageResponse {
        items: lessons.into_iter().map(Into::into).collect(),
        page,
        per_page,
        total,
        items_to_review: count(totals.items_to_review),
        cards_without_audio: count(totals.cards_without_audio),
    }))
}

pub(super) fn list_lessons_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("List lessons for review")
        .description(
            "The lessons in the order of the tracks, each with its unit and how its cards \
             stand: validated, flagged, to review and without a recording. \
             `itemsToReview` and `cardsWithoutAudio` count the whole content, for the review \
             queue's header.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<LessonSummaryPageResponse>, _>(|r| {
            r.description("One page of lessons")
        })
        .response_with::<400, ApiError, _>(|r| {
            r.description("A query parameter is not of the expected type")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<422, ApiError, _>(|r| r.description("`page` or `perPage` is out of range"))
}

/// Which page of a lesson's cards to list.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ListLessonCardsQuery {
    /// 1-based; 1 by default.
    #[validate(range(min = 1))]
    page: Option<u32>,
    /// 1 to 100; 50 by default.
    #[validate(range(min = 1, max = MAX_PER_PAGE))]
    per_page: Option<u32>,
}

/// One page of a lesson's cards, in order.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LessonCardPageResponse {
    items: Vec<CardResponse>,
    page: u32,
    per_page: u32,
    /// How many cards the lesson holds.
    total: u64,
}

async fn list_lesson_cards(
    State(state): State<AppState>,
    _: RequireRole<Admin>,
    ApiPath(path): ApiPath<LessonPath>,
    ValidQuery(query): ValidQuery<ListLessonCardsQuery>,
) -> Result<Json<LessonCardPageResponse>, ApiError> {
    let lesson_id = path.lesson_id;
    if !db::review::lesson_exists(&state.pool, lesson_id).await? {
        return Err(ApiError::NotFound);
    }
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(DEFAULT_PER_PAGE);
    let offset = i64::from(page - 1) * i64::from(per_page);
    let cards =
        db::review::cards_of_lesson(&state.pool, lesson_id, per_page.into(), offset).await?;
    let total = count(db::review::count_cards_of_lesson(&state.pool, lesson_id).await?);
    let usernames = content::usernames(
        &state.pool,
        cards.iter().flat_map(|card| card.review.authors()),
    )
    .await?;
    Ok(Json(LessonCardPageResponse {
        items: cards
            .into_iter()
            .map(|card| CardResponse::with(&usernames, card))
            .collect(),
        page,
        per_page,
        total,
    }))
}

fn list_lesson_cards_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("List a lesson's cards")
        .description("Every card of the lesson, whatever its status, in the lesson's order.")
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<LessonCardPageResponse>, _>(|r| {
            r.description("One page of cards")
        })
        .response_with::<400, ApiError, _>(|r| {
            r.description("A query parameter is not of the expected type")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<404, ApiError, _>(|r| r.description("No lesson has this id"))
        .response_with::<422, ApiError, _>(|r| r.description("`page` or `perPage` is out of range"))
}

/// The cards of the lesson to validate: those the reviewer has before
/// her eyes.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "cardIds": [
        "5f0c2b1e-8a3d-4c7e-9f61-2a4b6d8e0c13",
        "9a1d7c4e-2b6f-4e8a-b3c5-7d9e1f2a4b6c",
    ],
})))]
pub struct ValidateCardsRequest {
    /// 1 to 100 cards of the lesson.
    #[validate(length(min = 1, max = MAX_CARDS_PER_VALIDATION))]
    card_ids: Vec<Uuid>,
}

/// A card's review state after the validation.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CardReviewResponse {
    id: Uuid,
    status: StatusJson,
    validated_at: Option<DateTime<Utc>>,
    validated_by: Option<AuthorResponse>,
    /// Set on a flagged card, which the validation leaves to review.
    review_note: Option<String>,
}

/// Every card the request named, in the lesson's order.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CardValidationsResponse {
    cards: Vec<CardReviewResponse>,
}

async fn validate_cards(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ApiPath(path): ApiPath<LessonPath>,
    ValidJson(request): ValidJson<ValidateCardsRequest>,
) -> Result<Json<CardValidationsResponse>, ApiError> {
    let cards = review::validate_cards(&state.pool, &actor, path.lesson_id, &request.card_ids)
        .await
        .map_err(|refusal| match refusal {
            ContentRefusal::NotInParent => {
                let mut errors = ValidationErrors::new();
                errors.add("card_ids", ValidationError::new("card_not_in_lesson"));
                ApiError::Validation(errors)
            }
            other => other.into(),
        })?;
    let usernames =
        content::usernames(&state.pool, cards.iter().map(|card| card.validated_by)).await?;
    Ok(Json(CardValidationsResponse {
        cards: cards
            .into_iter()
            .map(|card: CardReview| CardReviewResponse {
                id: card.id,
                status: card.status.into(),
                validated_at: card.validated_at,
                validated_by: author(&usernames, card.validated_by),
                review_note: card.review_note,
            })
            .collect(),
    }))
}

fn validate_cards_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Validate a lesson's cards")
        .description(
            "Validates, in one transaction, the named cards that are to review without a \
             review note; a flagged card stays to review and a validated one keeps its \
             validation. Any `admin` validates, as one card at a time. It records nothing \
             but the cards' own validation, hence a `200` rather than a `201`. When an id is \
             not a card of the lesson, nothing is validated.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<CardValidationsResponse>, _>(|r| {
            r.description("The review state of every card named")
        })
        .response_with::<400, ApiError, _>(|r| {
            r.description("The body is not JSON of the expected shape")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<404, ApiError, _>(|r| r.description("No lesson has this id"))
        .response_with::<415, ApiError, _>(|r| r.description("The body is not `application/json`"))
        .response_with::<422, ApiError, _>(|r| {
            r.description(
                "`length`: no card or more than 100; `card_not_in_lesson`: an id is not a \
                 card of the lesson",
            )
        })
}
