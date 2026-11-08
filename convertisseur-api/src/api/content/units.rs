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
    Resource, ReviewResponse, TAG, TrackJson, created, delete, delete_docs, set_status,
    set_status_docs, validate_title,
};
use crate::{
    api::{
        SESSION_SECURITY,
        extract::{Admin, ApiPath, RequireRole, ValidJson},
    },
    content::{self, Track, Unit, UnitFields},
    db::{self, content::Units},
    error::ApiError,
    router::AppState,
};

pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route("/api/v1/units", post_with(create_unit, create_unit_docs))
        .api_route(
            "/api/v1/units/{unitId}",
            get_with(get_unit, get_unit_docs)
                .put_with(update_unit, update_unit_docs)
                .delete_with(delete::<UnitResource>, delete_docs::<UnitResource>),
        )
        .api_route(
            "/api/v1/units/{unitId}/status",
            put_with(set_status::<UnitResource>, set_status_docs::<UnitResource>),
        )
}

pub struct UnitResource;

impl Resource for UnitResource {
    type Rows = Units;
    type Path = UnitPath;
    const DELETE_SUMMARY: &'static str = "Delete a unit";
    const DELETE_DOCS: &'static str = "Deletes an empty unit; the next ones in its track move up. \
         Answers 204 when it is already gone.";
    const NOT_FOUND_DOCS: &'static str = "No unit has this id";
}

/// Names one unit in the path.
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnitPath {
    unit_id: Uuid,
}

impl From<UnitPath> for Uuid {
    fn from(path: UnitPath) -> Self {
        path.unit_id
    }
}

/// A new unit.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "track": "arabe",
    "title": "L'alphabet",
})))]
pub struct CreateUnitRequest {
    track: TrackJson,
    /// 1 to 200 characters.
    #[validate(custom(function = validate_title))]
    title: String,
    /// 1-based place among the track's units; last when absent or past the
    /// end.
    #[validate(range(min = 1))]
    position: Option<i32>,
}

/// A unit, rewritten whole. Any change sends it back to review.
#[derive(Deserialize, JsonSchema, Validate)]
#[serde(rename_all = "camelCase")]
#[schemars(extend("example" = serde_json::json!({
    "track": "arabe",
    "title": "L'alphabet",
    "position": 1,
})))]
pub struct UpdateUnitRequest {
    track: TrackJson,
    /// 1 to 200 characters.
    #[validate(custom(function = validate_title))]
    title: String,
    /// 1-based place among the track's units; last when past the end.
    #[validate(range(min = 1))]
    position: i32,
}

/// A unit of a track.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnitResponse {
    id: Uuid,
    track: TrackJson,
    /// 1-based place among the track's units.
    position: i32,
    title: String,
    #[serde(flatten)]
    review: ReviewResponse,
}

impl UnitResponse {
    async fn of(state: &AppState, unit: Unit) -> Result<Self, ApiError> {
        Ok(Self {
            id: unit.id,
            track: unit.track.into(),
            position: unit.position,
            title: unit.title,
            review: ReviewResponse::of(state, unit.review).await?,
        })
    }

    async fn find(state: &AppState, id: Uuid) -> Result<Self, ApiError> {
        let unit = db::content::find_unit(&state.pool, id)
            .await?
            .ok_or(ApiError::NotFound)?;
        Self::of(state, unit).await
    }
}

async fn create_unit(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ValidJson(request): ValidJson<CreateUnitRequest>,
) -> Result<Response, ApiError> {
    let fields = UnitFields {
        track: Track::from(request.track),
        title: request.title.trim().to_owned(),
    };
    let id = content::create::<Units>(
        &state.pool,
        &actor,
        fields.track,
        request.position,
        async |conn, position| db::content::insert_unit(conn, &fields, position, actor.id).await,
    )
    .await?;
    let unit = UnitResponse::find(&state, id).await?;
    Ok(created(format!("/api/v1/units/{id}"), unit))
}

fn create_unit_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Create a unit")
        .description("The unit starts `to_review`; the units after its position move down.")
        .security_requirement(SESSION_SECURITY)
        .response_with::<201, Json<UnitResponse>, _>(|r| r.description("The unit"))
        .response_with::<400, ApiError, _>(|r| {
            r.description("The body is not JSON of the expected shape")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<415, ApiError, _>(|r| r.description("The body is not `application/json`"))
        .response_with::<422, ApiError, _>(|r| {
            r.description("A field breaks a rule: `title_length`, `range`")
        })
}

async fn get_unit(
    State(state): State<AppState>,
    _: RequireRole<Admin>,
    ApiPath(path): ApiPath<UnitPath>,
) -> Result<Json<UnitResponse>, ApiError> {
    Ok(Json(UnitResponse::find(&state, path.unit_id).await?))
}

fn get_unit_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Get a unit")
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<UnitResponse>, _>(|r| r.description("The unit"))
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<404, ApiError, _>(|r| r.description(UnitResource::NOT_FOUND_DOCS))
}

async fn update_unit(
    State(state): State<AppState>,
    RequireRole { user: actor, .. }: RequireRole<Admin>,
    ApiPath(path): ApiPath<UnitPath>,
    ValidJson(request): ValidJson<UpdateUnitRequest>,
) -> Result<Json<UnitResponse>, ApiError> {
    let fields = UnitFields {
        track: Track::from(request.track),
        title: request.title.trim().to_owned(),
    };
    content::update::<Units>(
        &state.pool,
        &actor,
        path.unit_id,
        fields.track,
        request.position,
        async |conn, position| {
            db::content::update_unit(conn, path.unit_id, &fields, position, actor.id).await
        },
    )
    .await?;
    Ok(Json(UnitResponse::find(&state, path.unit_id).await?))
}

fn update_unit_docs(op: TransformOperation<'_>) -> TransformOperation<'_> {
    op.tag(TAG)
        .summary("Edit a unit")
        .description(
            "Rewrites the unit, possibly into another track or position. When anything \
             changes, it goes back to `to_review`; its lessons follow it.",
        )
        .security_requirement(SESSION_SECURITY)
        .response_with::<200, Json<UnitResponse>, _>(|r| r.description("The unit"))
        .response_with::<400, ApiError, _>(|r| {
            r.description("The body is not JSON of the expected shape")
        })
        .response_with::<401, ApiError, _>(|r| r.description("No live session"))
        .response_with::<403, ApiError, _>(|r| {
            r.description("`forbidden`: the account is a `user`")
        })
        .response_with::<404, ApiError, _>(|r| r.description(UnitResource::NOT_FOUND_DOCS))
        .response_with::<415, ApiError, _>(|r| r.description("The body is not `application/json`"))
        .response_with::<422, ApiError, _>(|r| {
            r.description("A field breaks a rule: `title_length`, `range`")
        })
}
