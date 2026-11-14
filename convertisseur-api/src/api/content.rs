//! The content endpoints: units, lessons and cards, and the review list
//! that spans all three. Every one of them is for an `admin` or the
//! `super_admin`.

mod audio;
mod cards;
mod lessons;
mod review;
mod units;

use std::collections::HashMap;

use aide::{
    axum::{ApiRouter, routing::get_with},
    transform::TransformOperation,
};
use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, NoContent, Response},
};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use super::{
    SESSION_SECURITY,
    extract::{Admin, ApiPath, RequireRole, ValidJson, ValidQuery},
};
use crate::{
    content::{
        self, Audio, ContentItem, ContentKind, ContentQuery, ContentRefusal, Review, Status,
        StatusChange, Track, Variety,
    },
    db::{self, content::Ordered},
    error::ApiError,
    letters::{FormColumns, Joining, LetterForms},
    router::AppState,
};

pub const TAG: &str = "Content";

const DEFAULT_PER_PAGE: u32 = 50;
const MAX_PER_PAGE: u32 = 100;
const MAX_TITLE_CHARS: usize = 200;
const MAX_TEXT_CHARS: usize = 500;
const MAX_REVIEW_NOTE_CHARS: usize = 2000;

pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route(
            "/api/v1/content-items",
            get_with(list_items, list_items_docs),
        )
        .merge(audio::routes())
        .merge(units::routes())
        .merge(lessons::routes())
        .merge(cards::routes())
        .merge(review::routes())
}

#[derive(Clone, Copy, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrackJson {
    Arabe,
    Islam,
}

impl From<TrackJson> for Track {
    fn from(track: TrackJson) -> Self {
        match track {
            TrackJson::Arabe => Self::Arabe,
            TrackJson::Islam => Self::Islam,
        }
    }
}

impl From<Track> for TrackJson {
    fn from(track: Track) -> Self {
        match track {
            Track::Arabe => Self::Arabe,
            Track::Islam => Self::Islam,
        }
    }
}

/// `to_review` content is never shown to learners.
#[derive(Clone, Copy, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatusJson {
    ToReview,
    Validated,
}

impl From<StatusJson> for Status {
    fn from(status: StatusJson) -> Self {
        match status {
            StatusJson::ToReview => Self::ToReview,
            StatusJson::Validated => Self::Validated,
        }
    }
}

impl From<Status> for StatusJson {
    fn from(status: Status) -> Self {
        match status {
            Status::ToReview => Self::ToReview,
            Status::Validated => Self::Validated,
        }
    }
}

/// Classical (and Quranic) Arabic, or Algerian darija.
#[derive(Clone, Copy, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VarietyJson {
    Classique,
    Darija,
}

impl From<VarietyJson> for Variety {
    fn from(variety: VarietyJson) -> Self {
        match variety {
            VarietyJson::Classique => Self::Classique,
            VarietyJson::Darija => Self::Darija,
        }
    }
}

impl From<Variety> for VarietyJson {
    fn from(variety: Variety) -> Self {
        match variety {
            Variety::Classique => Self::Classique,
            Variety::Darija => Self::Darija,
        }
    }
}

/// The shapes of a letter within a word, the tatweel `ـ` standing on the
/// side it joins; its isolated shape is the card's `arabic`. `kind` says
/// which shapes the letter has.
#[derive(Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[schemars(extend("example" = serde_json::json!({
    "kind": "joining",
    "initial": "بـ",
    "medial": "ـبـ",
    "final": "ـب",
})))]
pub enum LetterFormsResponse {
    /// Joins the letters on both sides.
    Joining {
        initial: String,
        medial: String,
        r#final: String,
    },
    /// Never joins the next letter (ا د ذ ر ز و): no initial or medial
    /// form.
    NonJoining { r#final: String },
    /// Only ever ends a word (ة ى): no initial or medial form.
    WordFinal { r#final: String },
}

impl LetterFormsResponse {
    /// The forms of the letter `arabic`, `None` for a card without them.
    pub fn of(arabic: &str, forms: FormColumns) -> Option<Self> {
        Some(match forms.into_forms()? {
            LetterForms::Joining {
                initial,
                medial,
                r#final,
            } => Self::Joining {
                initial,
                medial,
                r#final,
            },
            LetterForms::FinalOnly { r#final } => match Joining::of(arabic) {
                Some(Joining::WordFinal) => Self::WordFinal { r#final },
                _ => Self::NonJoining { r#final },
            },
        })
    }
}

#[derive(Clone, Copy, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum KindJson {
    Unit,
    Lesson,
    Card,
}

impl From<KindJson> for ContentKind {
    fn from(kind: KindJson) -> Self {
        match kind {
            KindJson::Unit => Self::Unit,
            KindJson::Lesson => Self::Lesson,
            KindJson::Card => Self::Card,
        }
    }
}

impl From<ContentKind> for KindJson {
    fn from(kind: ContentKind) -> Self {
        match kind {
            ContentKind::Unit => Self::Unit,
            ContentKind::Lesson => Self::Lesson,
            ContentKind::Card => Self::Card,
        }
    }
}

/// A recording a card or lesson holds. Play it from
/// `GET /api/v1/audio/{audioId}`.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AudioResponse {
    id: Uuid,
    /// The type the server recognised in the file, e.g. `audio/mpeg`.
    content_type: String,
    size_bytes: u64,
    /// `null` when the file does not give it cheaply: only WAV and Ogg do.
    duration_ms: Option<u32>,
    created_at: DateTime<Utc>,
}

impl From<Audio> for AudioResponse {
    fn from(audio: Audio) -> Self {
        Self {
            id: audio.id,
            content_type: audio.content_type,
            size_bytes: u64::try_from(audio.size_bytes).unwrap_or_default(),
            duration_ms: audio.duration_ms.and_then(|ms| u32::try_from(ms).ok()),
            created_at: audio.created_at,
        }
    }
}

/// An account, as content names its authors and reviewers.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthorResponse {
    id: Uuid,
    username: String,
}

fn author(usernames: &HashMap<Uuid, String>, id: Option<Uuid>) -> Option<AuthorResponse> {
    let id = id?;
    Some(AuthorResponse {
        id,
        username: usernames.get(&id)?.clone(),
    })
}

/// The review state every unit, lesson and card carries. An author is
/// `null` once their account is deleted.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewResponse {
    status: StatusJson,
    created_at: DateTime<Utc>,
    created_by: Option<AuthorResponse>,
    /// The last edit; an edit of content sends the item back to `to_review`,
    /// a move does not.
    updated_at: DateTime<Utc>,
    updated_by: Option<AuthorResponse>,
    /// Set while the item is `validated`.
    validated_at: Option<DateTime<Utc>>,
    validated_by: Option<AuthorResponse>,
    /// The key of the content file entry the item comes from, e.g.
    /// `islam/coran/al-fatiha/aya-3`; `null` for an item written in the admin
    /// area. An imported item's text and place change in the files alone.
    import_key: Option<String>,
    /// What a reviewer reports about the item; `null` when nothing is.
    review_note: Option<String>,
}

impl ReviewResponse {
    async fn of(state: &AppState, review: Review) -> Result<Self, ApiError> {
        let usernames = content::usernames(&state.pool, review.authors()).await?;
        Ok(Self::with(&usernames, review))
    }

    /// The review, its authors named from `usernames`.
    fn with(usernames: &HashMap<Uuid, String>, review: Review) -> Self {
        Self {
            status: review.status.into(),
            created_at: review.created_at,
            created_by: author(usernames, review.created_by),
            updated_at: review.updated_at,
            updated_by: author(usernames, review.updated_by),
            validated_at: review.validated_at,
            validated_by: author(usernames, review.validated_by),
            import_key: review.import_key,
            review_note: review.note,
        }
    }
}

impl From<ContentRefusal> for ApiError {
    fn from(refusal: ContentRefusal) -> Self {
        match refusal {
            ContentRefusal::NotFound => Self::NotFound,
            ContentRefusal::ParentNotFound => Self::Internal(anyhow::anyhow!(
                "a parent was not found where the request names none"
            )),
            ContentRefusal::NotEmpty => Self::NotEmpty,
            ContentRefusal::Imported => Self::Imported,
            ContentRefusal::NotInParent => Self::Internal(anyhow::anyhow!(
                "a row was not in its parent where the request names none"
            )),
            ContentRefusal::Failed(error) => Self::Internal(error),
        }
    }
}

/// Maps a refusal of a write that names a parent in `field`: a missing
/// parent breaks the rule `code` on it.
fn refused(refusal: ContentRefusal, field: &'static str, code: &'static str) -> ApiError {
    match refusal {
        ContentRefusal::ParentNotFound => {
            let mut errors = validator::ValidationErrors::new();
            errors.add(field, ValidationError::new(code));
            ApiError::Validation(errors)
        }
        other => other.into(),
    }
}

/// A title: 1 to 200 characters once trimmed.
fn validate_title(title: &str) -> Result<(), ValidationError> {
    let chars = title.trim().chars().count();
    if (1..=MAX_TITLE_CHARS).contains(&chars) {
        Ok(())
    } else {
        Err(ValidationError::new("title_length"))
    }
}

/// An optional text: at most 500 characters.
fn validate_text(text: &str) -> Result<(), ValidationError> {
    if text.trim().chars().count() <= MAX_TEXT_CHARS {
        Ok(())
    } else {
        Err(ValidationError::new("text_length"))
    }
}

/// A review note: at most 2000 characters once trimmed.
fn validate_review_note(note: &str) -> Result<(), ValidationError> {
    if note.trim().chars().count() <= MAX_REVIEW_NOTE_CHARS {
        Ok(())
    } else {
        Err(ValidationError::new("review_note_length"))
    }
}

/// Trims an optional text, and drops it when nothing is left.
fn optional_text(text: Option<String>) -> Option<String> {
    text.map(|t| t.trim().to_owned()).filter(|t| !t.is_empty())
}

/// A 201 with the new item and its address.
fn created(location: String, body: impl Serialize) -> Response {
    (
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(body),
    )
        .into_response()
}

/// An endpoint family over one kind of content: its rows, the path that
/// names one, and how its docs call it.
pub trait Resource: Send + Sync + 'static {
    type Rows: Ordered;
    type Path: DeserializeOwned + JsonSchema + Send + Into<Uuid>;
    const DELETE_SUMMARY: &'static str;
    const DELETE_DOCS: &'static str;
    const NOT_FOUND_DOCS: &'static str;
}

/// The review status to set.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({ "status": "validated" })))]
pub struct SetStatusRequest {
    /// `validated` shows the item to learners; `to_review` hides it again.
    status: StatusJson,
}

/// An item's review status.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    status: StatusJson,
    validated_at: Option<DateTime<Utc>>,
    validated_by: Option<AuthorResponse>,
}

async fn set_status<R: Resource>(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ApiPath(path): ApiPath<R::Path>,
    ValidJson(request): ValidJson<SetStatusRequest>,
) -> Result<Json<StatusResponse>, ApiError> {
    let StatusChange {
        status,
        validated_at,
        validated_by,
    } = content::set_status::<R::Rows>(&state.pool, &actor, path.into(), request.status.into())
        .await?;
    let usernames = content::usernames(&state.pool, [validated_by]).await?;
    Ok(Json(StatusResponse {
        status: status.into(),
        validated_at,
        validated_by: author(&usernames, validated_by),
    }))
}

fn set_status_docs<R: Resource>(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Set the review status")
        .description(
            "Any `admin` validates, its author included, or sends back to review. Setting \
             the status the item already has changes nothing.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<StatusResponse>, _>(|r| r.description("The review status"))
        .response_with::<400, ApiError, _>(|r| {
            r.description("The body is not JSON of the expected shape")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<404, ApiError, _>(|r| r.description(R::NOT_FOUND_DOCS))
        .response_with::<415, ApiError, _>(|r| r.description("The body is not `application/json`"))
}

/// The review note to keep on an item.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "text": "Vocalisation fausse sur le 2e mot",
})))]
pub struct SetReviewNoteRequest {
    /// At most 2000 characters; empty, blank or `null` clears the note.
    #[validate(custom(function = validate_review_note))]
    text: Option<String>,
}

/// An item's review note.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewNoteResponse {
    /// `null` when the item has none.
    text: Option<String>,
}

async fn set_review_note<R: Resource>(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ApiPath(path): ApiPath<R::Path>,
    ValidJson(request): ValidJson<SetReviewNoteRequest>,
) -> Result<Json<ReviewNoteResponse>, ApiError> {
    let text = content::set_review_note::<R::Rows>(
        &state.pool,
        &actor,
        path.into(),
        optional_text(request.text),
    )
    .await?;
    Ok(Json(ReviewNoteResponse { text }))
}

fn set_review_note_docs<R: Resource>(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Set the review note")
        .description(
            "Keeps what a reviewer reports about the item, e.g. a wrong vowel, whether it is \
             imported or not. The note leaves the review status as it is. When the content \
             files rewrite an imported item, its note is cleared.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<ReviewNoteResponse>, _>(|r| r.description("The review note"))
        .response_with::<400, ApiError, _>(|r| {
            r.description("The body is not JSON of the expected shape")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<404, ApiError, _>(|r| r.description(R::NOT_FOUND_DOCS))
        .response_with::<415, ApiError, _>(|r| r.description("The body is not `application/json`"))
        .response_with::<422, ApiError, _>(|r| r.description("`review_note_length`: too long"))
}

async fn delete<R: Resource>(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ApiPath(path): ApiPath<R::Path>,
) -> Result<NoContent, ApiError> {
    content::delete::<R::Rows>(&state.pool, &state.storage, &actor, path.into()).await?;
    Ok(NoContent)
}

fn delete_docs<R: Resource>(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary(R::DELETE_SUMMARY)
        .description(R::DELETE_DOCS)
        .security_requirement(SESSION_SECURITY)
        .response_with::<204, NoContent, _>(|r| r.description("It is gone"))
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<409, ApiError, _>(|r| {
            r.description(
                "`imported-content`: it comes from the content files, which alone delete it; \
                 `not-empty`: it still holds lessons or cards",
            )
        })
}

/// Which items to list, and which page of them.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ListItemsQuery {
    /// 1-based; 1 by default.
    #[validate(range(min = 1))]
    page: Option<u32>,
    /// 1 to 100; 50 by default.
    #[validate(range(min = 1, max = MAX_PER_PAGE))]
    per_page: Option<u32>,
    status: Option<StatusJson>,
    track: Option<TrackJson>,
    kind: Option<KindJson>,
    /// Part of a title or of a card's text, or of the title of its parent (a
    /// lesson's unit, a card's lesson), compared case-insensitively; Arabic
    /// short vowels and other diacritics are ignored on both sides.
    q: Option<String>,
    /// `true` lists only the items with a review note, `false` only those
    /// without.
    has_review_note: Option<bool>,
}

/// A unit, lesson or card in the review list.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContentItemResponse {
    kind: KindJson,
    id: Uuid,
    track: TrackJson,
    /// A card's title is its Arabic.
    title: String,
    /// A card's transliteration; `null` for a unit or a lesson.
    transliteration: Option<String>,
    /// The unit of a lesson, the lesson of a card; `null` for a unit.
    parent_title: Option<String>,
    status: StatusJson,
    updated_at: DateTime<Utc>,
    /// The key of the content file entry the item comes from; `null` for an
    /// item written in the admin area.
    import_key: Option<String>,
    review_note: Option<String>,
    /// The unit of a lesson or of a card; `null` for a unit.
    unit_id: Option<Uuid>,
    /// The lesson of a card; `null` for a unit or a lesson.
    lesson_id: Option<Uuid>,
}

impl From<ContentItem> for ContentItemResponse {
    fn from(item: ContentItem) -> Self {
        Self {
            kind: item.kind.into(),
            id: item.id,
            track: item.track.into(),
            title: item.title,
            transliteration: item.transliteration,
            parent_title: item.parent_title,
            status: item.status.into(),
            updated_at: item.updated_at,
            import_key: item.import_key,
            review_note: item.review_note,
            unit_id: item.unit_id,
            lesson_id: item.lesson_id,
        }
    }
}

/// How many items match the filters other than `status`, per status.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatusCountsResponse {
    to_review: u64,
    validated: u64,
}

/// One page of the review list, the most recently edited first.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContentItemPageResponse {
    items: Vec<ContentItemResponse>,
    page: u32,
    per_page: u32,
    /// How many items match the filters, across all pages.
    total: u64,
    status_counts: StatusCountsResponse,
    /// How many items match every filter but `hasReviewNote` and carry a
    /// review note.
    review_note_count: u64,
}

async fn list_items(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ValidQuery(query): ValidQuery<ListItemsQuery>,
) -> Result<Json<ContentItemPageResponse>, ApiError> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(DEFAULT_PER_PAGE);
    let filter = ContentQuery {
        status: query.status.map(Status::from),
        track: query.track.map(Track::from),
        kind: query.kind.map(ContentKind::from),
        text: query.q.as_deref().map(str::trim).filter(|q| !q.is_empty()),
        has_review_note: query.has_review_note,
    };
    let offset = i64::from(page - 1) * i64::from(per_page);
    let counts = db::content::counts(&state.pool, &filter).await?;
    let count = |status: Option<Status>, noted: Option<bool>| -> u64 {
        counts
            .iter()
            .filter(|(s, n, _)| status.is_none_or(|w| w == *s) && noted.is_none_or(|w| w == *n))
            .map(|(_, _, items)| u64::try_from(*items).unwrap_or_default())
            .sum()
    };
    let status_counts = StatusCountsResponse {
        to_review: count(Some(Status::ToReview), filter.has_review_note),
        validated: count(Some(Status::Validated), filter.has_review_note),
    };
    let review_note_count = count(filter.status, Some(true));
    let total = count(filter.status, filter.has_review_note);
    let items = db::content::list(&state.pool, &filter, per_page.into(), offset).await?;
    tracing::debug!(actor_id = %actor.id, page, per_page, total, "content listed");
    Ok(Json(ContentItemPageResponse {
        items: items.into_iter().map(Into::into).collect(),
        page,
        per_page,
        total,
        status_counts,
        review_note_count,
    }))
}

fn list_items_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("List content for review")
        .description(
            "Units, lessons and cards together, the most recently edited first. \
             `statusCounts` counts the items matching every filter but `status`, and \
             `reviewNoteCount` those with a review note matching every filter but \
             `hasReviewNote`.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<ContentItemPageResponse>, _>(|r| {
            r.description("One page of items")
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
