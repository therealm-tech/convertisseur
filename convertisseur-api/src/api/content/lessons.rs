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
use validator::Validate;

use super::{
    Resource, ReviewResponse, TAG, TrackJson, created, delete, delete_docs, refused, set_status,
    set_status_docs, validate_title,
};
use crate::{
    api::{
        SESSION_SECURITY,
        extract::{Admin, ApiPath, RequireRole, ValidJson},
    },
    content::{self, Lesson, LessonFields},
    db::{self, content::Lessons},
    error::ApiError,
    router::AppState,
};

pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route(
            "/api/v1/lessons",
            post_with(create_lesson, create_lesson_docs),
        )
        .api_route(
            "/api/v1/lessons/{lessonId}",
            get_with(get_lesson, get_lesson_docs)
                .put_with(update_lesson, update_lesson_docs)
                .delete_with(delete::<LessonResource>, delete_docs::<LessonResource>),
        )
        .api_route(
            "/api/v1/lessons/{lessonId}/status",
            put_with(
                set_status::<LessonResource>,
                set_status_docs::<LessonResource>,
            ),
        )
}

pub struct LessonResource;

impl Resource for LessonResource {
    type Rows = Lessons;
    type Path = LessonPath;
    const DELETE_SUMMARY: &'static str = "Delete a lesson";
    const DELETE_DOCS: &'static str = "Deletes a lesson with no cards; the next ones in its unit \
         move up. Answers 204 when it is already gone.";
    const NOT_FOUND_DOCS: &'static str = "No lesson has this id";
}

/// Names one lesson in the path.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LessonPath {
    lesson_id: Uuid,
}

impl From<LessonPath> for Uuid {
    fn from(path: LessonPath) -> Self {
        path.lesson_id
    }
}

/// A new lesson.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "unitId": "0b8e6a8c-4d2f-4f0a-9a57-3d1c8f1e2b6d",
    "title": "Les 4 formes des lettres",
})))]
pub struct CreateLessonRequest {
    unit_id: Uuid,
    /// 1 to 200 characters.
    #[validate(custom(function = validate_title))]
    title: String,
    /// 1-based place among the unit's lessons; last when absent or past the
    /// end.
    #[validate(range(min = 1))]
    position: Option<i32>,
}

/// A lesson, rewritten whole. Any change sends it back to review.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "unitId": "0b8e6a8c-4d2f-4f0a-9a57-3d1c8f1e2b6d",
    "title": "Les 4 formes des lettres",
    "position": 1,
})))]
pub struct UpdateLessonRequest {
    unit_id: Uuid,
    /// 1 to 200 characters.
    #[validate(custom(function = validate_title))]
    title: String,
    /// 1-based place among the unit's lessons; last when past the end.
    #[validate(range(min = 1))]
    position: i32,
}

/// The unit a lesson belongs to.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LessonUnitResponse {
    id: Uuid,
    position: i32,
    title: String,
}

/// A lesson of a unit.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LessonResponse {
    id: Uuid,
    unit: LessonUnitResponse,
    /// The track of its unit.
    track: TrackJson,
    /// 1-based place among the unit's lessons.
    position: i32,
    title: String,
    #[serde(flatten)]
    review: ReviewResponse,
}

impl LessonResponse {
    async fn of(state: &AppState, lesson: Lesson) -> Result<Self, ApiError> {
        Ok(Self {
            id: lesson.id,
            unit: LessonUnitResponse {
                id: lesson.unit_id,
                position: lesson.unit_position,
                title: lesson.unit_title,
            },
            track: lesson.track.into(),
            position: lesson.position,
            title: lesson.title,
            review: ReviewResponse::of(state, lesson.review).await?,
        })
    }

    async fn find(state: &AppState, id: Uuid) -> Result<Self, ApiError> {
        let lesson = db::content::find_lesson(&state.pool, id)
            .await?
            .ok_or(ApiError::NotFound)?;
        Self::of(state, lesson).await
    }
}

async fn create_lesson(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ValidJson(request): ValidJson<CreateLessonRequest>,
) -> Result<Response, ApiError> {
    let fields = LessonFields {
        unit_id: request.unit_id,
        title: request.title.trim().to_owned(),
    };
    let id = content::create::<Lessons>(
        &state.pool,
        &actor,
        fields.unit_id,
        request.position,
        async |conn, position| db::content::insert_lesson(conn, &fields, position, actor.id).await,
    )
    .await
    .map_err(|refusal| refused(refusal, "unit_id", "unit_not_found"))?;
    let lesson = LessonResponse::find(&state, id).await?;
    Ok(created(format!("/api/v1/lessons/{id}"), lesson))
}

fn create_lesson_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Create a lesson")
        .description("The lesson starts `to_review`; the lessons after its position move down.")
        .security_requirement(SESSION_SECURITY)
        .response_with::<201, Json<LessonResponse>, _>(|r| r.description("The lesson"))
        .response_with::<400, ApiError, _>(|r| {
            r.description("The body is not JSON of the expected shape")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<415, ApiError, _>(|r| r.description("The body is not `application/json`"))
        .response_with::<422, ApiError, _>(|r| {
            r.description("A field breaks a rule: `title_length`, `range`, `unit_not_found`")
        })
}

async fn get_lesson(
    State(state): State<AppState>,
    _: RequireRole<Admin>,
    ApiPath(path): ApiPath<LessonPath>,
) -> Result<Json<LessonResponse>, ApiError> {
    Ok(Json(LessonResponse::find(&state, path.lesson_id).await?))
}

fn get_lesson_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Get a lesson")
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<LessonResponse>, _>(|r| r.description("The lesson"))
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<404, ApiError, _>(|r| r.description(LessonResource::NOT_FOUND_DOCS))
}

async fn update_lesson(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ApiPath(path): ApiPath<LessonPath>,
    ValidJson(request): ValidJson<UpdateLessonRequest>,
) -> Result<Json<LessonResponse>, ApiError> {
    let fields = LessonFields {
        unit_id: request.unit_id,
        title: request.title.trim().to_owned(),
    };
    content::update::<Lessons>(
        &state.pool,
        &actor,
        path.lesson_id,
        fields.unit_id,
        request.position,
        async |conn, position| {
            db::content::update_lesson(conn, path.lesson_id, &fields, position, actor.id).await
        },
    )
    .await
    .map_err(|refusal| refused(refusal, "unit_id", "unit_not_found"))?;
    Ok(Json(LessonResponse::find(&state, path.lesson_id).await?))
}

fn update_lesson_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Edit a lesson")
        .description(
            "Rewrites the lesson, possibly into another unit or position. When anything \
             changes, it goes back to `to_review`; its cards follow it.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<LessonResponse>, _>(|r| r.description("The lesson"))
        .response_with::<400, ApiError, _>(|r| {
            r.description("The body is not JSON of the expected shape")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<404, ApiError, _>(|r| r.description(LessonResource::NOT_FOUND_DOCS))
        .response_with::<415, ApiError, _>(|r| r.description("The body is not `application/json`"))
        .response_with::<422, ApiError, _>(|r| {
            r.description("A field breaks a rule: `title_length`, `range`, `unit_not_found`")
        })
}
