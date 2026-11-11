use std::collections::HashMap;

use aide::{
    axum::{
        ApiRouter,
        routing::{get_with, post_with, put_with},
    },
    transform::TransformOperation,
};
use axum::{Json, extract::State, response::Response};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use super::{
    AudioResponse, LetterFormsResponse, MAX_TEXT_CHARS, Resource, ReviewResponse, TAG, TrackJson,
    VarietyJson,
    audio::{delete_audio, delete_audio_docs, put_audio, put_audio_docs, upload_limit},
    created, delete, delete_docs, optional_text, refused, set_review_note, set_review_note_docs,
    set_status, set_status_docs, validate_text,
};
use crate::{
    api::{
        SESSION_SECURITY,
        extract::{Admin, ApiPath, RequireRole, ValidJson},
    },
    content::{self, Card, CardFields, Variety},
    db::{self, content::Cards},
    error::ApiError,
    router::AppState,
};

pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route("/api/v1/cards", post_with(create_card, create_card_docs))
        .api_route(
            "/api/v1/cards/{cardId}",
            get_with(get_card, get_card_docs)
                .put_with(update_card, update_card_docs)
                .delete_with(delete::<CardResource>, delete_docs::<CardResource>),
        )
        .api_route(
            "/api/v1/cards/{cardId}/audio",
            put_with(put_audio::<CardResource>, put_audio_docs::<CardResource>)
                .delete_with(
                    delete_audio::<CardResource>,
                    delete_audio_docs::<CardResource>,
                )
                .layer(upload_limit()),
        )
        .api_route(
            "/api/v1/cards/{cardId}/status",
            put_with(set_status::<CardResource>, set_status_docs::<CardResource>),
        )
        .api_route(
            "/api/v1/cards/{cardId}/review-note",
            put_with(
                set_review_note::<CardResource>,
                set_review_note_docs::<CardResource>,
            ),
        )
}

pub struct CardResource;

impl Resource for CardResource {
    type Rows = Cards;
    type Path = CardPath;
    const DELETE_SUMMARY: &'static str = "Delete a card";
    const DELETE_DOCS: &'static str = "Deletes the card; the next ones in its lesson move up. \
         Answers 204 when it is already gone.";
    const NOT_FOUND_DOCS: &'static str = "No card has this id";
}

/// Names one card in the path.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CardPath {
    card_id: Uuid,
}

impl From<CardPath> for Uuid {
    fn from(path: CardPath) -> Self {
        path.card_id
    }
}

/// Vocalised Arabic: 1 to 500 characters, at least one of them in the
/// Arabic script.
fn validate_arabic(arabic: &str) -> Result<(), ValidationError> {
    let arabic = arabic.trim();
    if !(1..=MAX_TEXT_CHARS).contains(&arabic.chars().count()) {
        return Err(ValidationError::new("arabic_length"));
    }
    if !arabic
        .chars()
        .any(|c| ('\u{0600}'..='\u{06FF}').contains(&c))
    {
        return Err(ValidationError::new("arabic_script"));
    }
    Ok(())
}

/// A new card.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "lessonId": "5f0c2b1e-8a3d-4c7e-9f61-2a4b6d8e0c13",
    "arabic": "شُكْرًا",
    "transliteration": "shukran",
    "meaningFr": "merci",
    "variety": "classique",
})))]
pub struct CreateCardRequest {
    lesson_id: Uuid,
    /// The vocalised Arabic: 1 to 500 characters, some in the Arabic
    /// script.
    #[validate(custom(function = validate_arabic))]
    arabic: String,
    /// At most 500 characters; empty is `null`.
    #[validate(custom(function = validate_text))]
    transliteration: Option<String>,
    /// The meaning in French; at most 500 characters, empty is `null`.
    #[validate(custom(function = validate_text))]
    meaning_fr: Option<String>,
    variety: VarietyJson,
    /// 1-based place among the lesson's cards; last when absent or past the
    /// end.
    #[validate(range(min = 1))]
    position: Option<i32>,
}

/// A card, rewritten whole: an absent text is cleared. Any change of content
/// sends it back to review; a move keeps its status.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "lessonId": "5f0c2b1e-8a3d-4c7e-9f61-2a4b6d8e0c13",
    "arabic": "شُكْرًا",
    "transliteration": "shukran",
    "meaningFr": "merci",
    "variety": "classique",
    "position": 1,
})))]
pub struct UpdateCardRequest {
    lesson_id: Uuid,
    /// The vocalised Arabic: 1 to 500 characters, some in the Arabic
    /// script.
    #[validate(custom(function = validate_arabic))]
    arabic: String,
    /// At most 500 characters; empty or absent is `null`.
    #[validate(custom(function = validate_text))]
    transliteration: Option<String>,
    /// The meaning in French; at most 500 characters, empty or absent is
    /// `null`.
    #[validate(custom(function = validate_text))]
    meaning_fr: Option<String>,
    variety: VarietyJson,
    /// 1-based place among the lesson's cards; last when past the end.
    #[validate(range(min = 1))]
    position: i32,
}

/// The lesson a card belongs to.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CardLessonResponse {
    id: Uuid,
    title: String,
}

/// A card of a lesson.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CardResponse {
    id: Uuid,
    lesson: CardLessonResponse,
    /// The track of its lesson's unit.
    track: TrackJson,
    /// 1-based place among the lesson's cards.
    position: i32,
    arabic: String,
    transliteration: Option<String>,
    meaning_fr: Option<String>,
    variety: VarietyJson,
    /// The other shapes of the letter, for a card of the alphabet; `null`
    /// for any other card. Only the content files write them.
    forms: Option<LetterFormsResponse>,
    /// `null` when the card has no recording.
    audio: Option<AudioResponse>,
    #[serde(flatten)]
    review: ReviewResponse,
}

impl CardResponse {
    async fn of(state: &AppState, card: Card) -> Result<Self, ApiError> {
        let usernames = content::usernames(&state.pool, card.review.authors()).await?;
        Ok(Self::with(&usernames, card))
    }

    /// The card, its authors named from `usernames`.
    pub(super) fn with(usernames: &HashMap<Uuid, String>, card: Card) -> Self {
        Self {
            id: card.id,
            lesson: CardLessonResponse {
                id: card.lesson_id,
                title: card.lesson_title,
            },
            track: card.track.into(),
            position: card.position,
            forms: LetterFormsResponse::of(&card.arabic, card.forms),
            arabic: card.arabic,
            transliteration: card.transliteration,
            meaning_fr: card.meaning_fr,
            variety: card.variety.into(),
            audio: card.audio.into_audio().map(Into::into),
            review: ReviewResponse::with(usernames, card.review),
        }
    }

    async fn find(state: &AppState, id: Uuid) -> Result<Self, ApiError> {
        let card = db::content::find_card(&state.pool, id)
            .await?
            .ok_or(ApiError::NotFound)?;
        Self::of(state, card).await
    }
}

async fn create_card(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ValidJson(request): ValidJson<CreateCardRequest>,
) -> Result<Response, ApiError> {
    let fields = CardFields {
        lesson_id: request.lesson_id,
        arabic: request.arabic.trim().to_owned(),
        transliteration: optional_text(request.transliteration),
        meaning_fr: optional_text(request.meaning_fr),
        variety: Variety::from(request.variety),
    };
    let id = content::create::<Cards>(
        &state.pool,
        &actor,
        fields.lesson_id,
        request.position,
        async |conn, position| {
            db::content::insert_card(conn, &fields, None, position, None, actor.id).await
        },
    )
    .await
    .map_err(|refusal| refused(refusal, "lesson_id", "lesson_not_found"))?;
    let card = CardResponse::find(&state, id).await?;
    Ok(created(format!("/api/v1/cards/{id}"), card))
}

fn create_card_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Create a card")
        .description("The card starts `to_review`; the cards after its position move down.")
        .security_requirement(SESSION_SECURITY)
        .response_with::<201, Json<CardResponse>, _>(|r| r.description("The card"))
        .response_with::<400, ApiError, _>(|r| {
            r.description("The body is not JSON of the expected shape")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<415, ApiError, _>(|r| r.description("The body is not `application/json`"))
        .response_with::<422, ApiError, _>(|r| {
            r.description(
                "A field breaks a rule: `arabic_length`, `arabic_script`, `text_length`, \
                 `range`, `lesson_not_found`",
            )
        })
}

async fn get_card(
    State(state): State<AppState>,
    _: RequireRole<Admin>,
    ApiPath(path): ApiPath<CardPath>,
) -> Result<Json<CardResponse>, ApiError> {
    Ok(Json(CardResponse::find(&state, path.card_id).await?))
}

fn get_card_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Get a card")
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<CardResponse>, _>(|r| r.description("The card"))
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<404, ApiError, _>(|r| r.description(CardResource::NOT_FOUND_DOCS))
}

async fn update_card(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ApiPath(path): ApiPath<CardPath>,
    ValidJson(request): ValidJson<UpdateCardRequest>,
) -> Result<Json<CardResponse>, ApiError> {
    let fields = CardFields {
        lesson_id: request.lesson_id,
        arabic: request.arabic.trim().to_owned(),
        transliteration: optional_text(request.transliteration),
        meaning_fr: optional_text(request.meaning_fr),
        variety: Variety::from(request.variety),
    };
    content::update::<Cards>(
        &state.pool,
        &actor,
        path.card_id,
        fields.lesson_id,
        request.position,
        async |conn, position| {
            db::content::update_card(conn, path.card_id, &fields, position, actor.id).await
        },
    )
    .await
    .map_err(|refusal| refused(refusal, "lesson_id", "lesson_not_found"))?;
    Ok(Json(CardResponse::find(&state, path.card_id).await?))
}

fn update_card_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Edit a card")
        .description(
            "Rewrites the card, possibly into another lesson or position. A change of \
             content sends it back to `to_review`; a move alone keeps its status.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<CardResponse>, _>(|r| r.description("The card"))
        .response_with::<400, ApiError, _>(|r| {
            r.description("The body is not JSON of the expected shape")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<404, ApiError, _>(|r| r.description(CardResource::NOT_FOUND_DOCS))
        .response_with::<409, ApiError, _>(|r| {
            r.description(
                "`imported-content`: it comes from the content files, which alone change it",
            )
        })
        .response_with::<415, ApiError, _>(|r| r.description("The body is not `application/json`"))
        .response_with::<422, ApiError, _>(|r| {
            r.description(
                "A field breaks a rule: `arabic_length`, `arabic_script`, `text_length`, \
                 `range`, `lesson_not_found`",
            )
        })
}
