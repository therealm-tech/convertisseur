//! Learning content: units, lessons and cards, their order among their
//! siblings and their review. Every write runs in one transaction under the
//! content lock.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor, PgPool};
use uuid::Uuid;

use crate::{
    db::{self, content::Ordered},
    users::User,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "content_track", rename_all = "snake_case")]
pub enum Track {
    Arabe,
    Islam,
}

/// Content to review is never shown to learners.
#[derive(Clone, Copy, Debug, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "content_status", rename_all = "snake_case")]
pub enum Status {
    ToReview,
    Validated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "arabic_variety", rename_all = "snake_case")]
pub enum Variety {
    Classique,
    Darija,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "content_kind", rename_all = "snake_case")]
pub enum ContentKind {
    Unit,
    Lesson,
    Card,
}

/// Who wrote a row, who last changed it, and who validated it.
#[derive(sqlx::FromRow)]
pub struct Review {
    pub status: Status,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
    pub validated_at: Option<DateTime<Utc>>,
    pub validated_by: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
pub struct Unit {
    pub id: Uuid,
    pub track: Track,
    pub position: i32,
    pub title: String,
    #[sqlx(flatten)]
    pub review: Review,
}

#[derive(sqlx::FromRow)]
pub struct Lesson {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub unit_position: i32,
    pub unit_title: String,
    pub track: Track,
    pub position: i32,
    pub title: String,
    #[sqlx(flatten)]
    pub review: Review,
}

#[derive(sqlx::FromRow)]
pub struct Card {
    pub id: Uuid,
    pub lesson_id: Uuid,
    pub lesson_title: String,
    pub track: Track,
    pub position: i32,
    pub arabic: String,
    pub transliteration: Option<String>,
    pub meaning_fr: Option<String>,
    pub variety: Variety,
    pub audio_id: Option<Uuid>,
    pub audio_content_type: Option<String>,
    pub audio_size_bytes: Option<i64>,
    pub audio_duration_ms: Option<i32>,
    pub audio_created_at: Option<DateTime<Utc>>,
    #[sqlx(flatten)]
    pub review: Review,
}

/// What an author writes in a unit; its position is set apart.
pub struct UnitFields {
    pub track: Track,
    pub title: String,
}

pub struct LessonFields {
    pub unit_id: Uuid,
    pub title: String,
}

pub struct CardFields {
    pub lesson_id: Uuid,
    pub arabic: String,
    pub transliteration: Option<String>,
    pub meaning_fr: Option<String>,
    pub variety: Variety,
}

/// Which items the review list shows: `None` matches everything.
pub struct ContentQuery<'a> {
    pub status: Option<Status>,
    pub track: Option<Track>,
    pub kind: Option<ContentKind>,
    /// Part of the title or the text, compared case-insensitively and
    /// without Arabic diacritics.
    pub text: Option<&'a str>,
}

/// A unit, lesson or card as the review list shows it.
#[derive(sqlx::FromRow)]
pub struct ContentItem {
    pub kind: ContentKind,
    pub id: Uuid,
    pub track: Track,
    pub title: String,
    pub transliteration: Option<String>,
    pub parent_title: Option<String>,
    pub status: Status,
    pub updated_at: DateTime<Utc>,
}

/// The review state of a row after a status change.
#[derive(sqlx::FromRow)]
pub struct StatusChange {
    pub status: Status,
    pub validated_at: Option<DateTime<Utc>>,
    pub validated_by: Option<Uuid>,
}

/// Why a content operation did not happen.
#[derive(Debug)]
pub enum ContentRefusal {
    /// No row has this id.
    NotFound,
    /// The unit or lesson to attach the row to does not exist.
    ParentNotFound,
    /// The unit still has lessons, or the lesson still has cards.
    NotEmpty,
    Failed(anyhow::Error),
}

impl From<sqlx::Error> for ContentRefusal {
    fn from(error: sqlx::Error) -> Self {
        Self::Failed(error.into())
    }
}

impl From<anyhow::Error> for ContentRefusal {
    fn from(error: anyhow::Error) -> Self {
        Self::Failed(error)
    }
}

/// Makes room for a row under `parent` and returns its position: the one
/// requested, kept between first and last, or last. `moving` is the row
/// itself when it is already in the table.
async fn place<O: Ordered>(
    conn: &mut PgConnection,
    parent: O::Parent,
    requested: Option<i32>,
    moving: Option<Uuid>,
) -> Result<i32, ContentRefusal> {
    if !db::content::parent_exists::<O>(&mut *conn, parent).await? {
        tracing::debug!(kind = ?O::KIND, "parent not found");
        return Err(ContentRefusal::ParentNotFound);
    }
    let siblings = db::content::count_siblings::<O>(&mut *conn, parent, moving).await?;
    let last = i32::try_from(siblings).map_or(i32::MAX, |n| n.saturating_add(1));
    let position = requested.map_or(last, |p| p.clamp(1, last));
    db::content::shift::<O>(&mut *conn, parent, position, 1, moving).await?;
    Ok(position)
}

/// Creates a row under `parent`, at the requested position or last. `insert`
/// writes it at the position it is given and returns its id.
pub async fn create<O: Ordered>(
    pool: &PgPool,
    actor: &User,
    parent: O::Parent,
    requested: Option<i32>,
    insert: impl AsyncFnOnce(&mut PgConnection, i32) -> sqlx::Result<Uuid>,
) -> Result<Uuid, ContentRefusal> {
    let mut tx = pool.begin().await?;
    db::content::lock(&mut *tx).await?;
    let position = place::<O>(&mut tx, parent, requested, None).await?;
    let id = insert(&mut tx, position).await?;
    tx.commit().await?;
    tracing::info!(kind = ?O::KIND, %id, actor_id = %actor.id, position, "content created");
    Ok(id)
}

/// Rewrites a row, possibly under another parent or at another position,
/// and closes the gap it leaves. `write` stores the fields at the position
/// it is given, and returns whether one of them changed, which sends the row
/// back to review.
pub async fn update<O: Ordered>(
    pool: &PgPool,
    actor: &User,
    id: Uuid,
    parent: O::Parent,
    requested: i32,
    write: impl AsyncFnOnce(&mut PgConnection, i32) -> sqlx::Result<bool>,
) -> Result<(), ContentRefusal> {
    let mut tx = pool.begin().await?;
    db::content::lock(&mut *tx).await?;
    let Some(current) = db::content::lock_row::<O>(&mut *tx, id).await? else {
        return Err(ContentRefusal::NotFound);
    };
    db::content::shift::<O>(
        &mut *tx,
        current.parent,
        current.position.saturating_add(1),
        -1,
        Some(id),
    )
    .await?;
    let position = place::<O>(&mut tx, parent, Some(requested), Some(id)).await?;
    let changed = write(&mut tx, position).await?;
    tx.commit().await?;
    if changed {
        tracing::info!(kind = ?O::KIND, %id, actor_id = %actor.id, position, "content edited, back to review");
    } else {
        tracing::debug!(kind = ?O::KIND, %id, actor_id = %actor.id, "content unchanged");
    }
    Ok(())
}

/// Deletes a row and closes the gap it leaves; succeeds when it is already
/// gone. A unit with lessons or a lesson with cards stays.
pub async fn delete<O: Ordered>(
    pool: &PgPool,
    actor: &User,
    id: Uuid,
) -> Result<(), ContentRefusal> {
    let mut tx = pool.begin().await?;
    db::content::lock(&mut *tx).await?;
    if db::content::has_children::<O>(&mut *tx, id).await? {
        tracing::debug!(kind = ?O::KIND, %id, "deletion refused: not empty");
        return Err(ContentRefusal::NotEmpty);
    }
    let Some(placement) = db::content::delete::<O>(&mut *tx, id).await? else {
        tracing::debug!(kind = ?O::KIND, %id, "already deleted");
        return Ok(());
    };
    db::content::shift::<O>(
        &mut *tx,
        placement.parent,
        placement.position.saturating_add(1),
        -1,
        None,
    )
    .await?;
    tx.commit().await?;
    tracing::info!(kind = ?O::KIND, %id, actor_id = %actor.id, "content deleted");
    Ok(())
}

/// Validates a row, or sends it back to review. Setting the status it
/// already has changes nothing.
pub async fn set_status<O: Ordered>(
    pool: &PgPool,
    actor: &User,
    id: Uuid,
    status: Status,
) -> Result<StatusChange, ContentRefusal> {
    let validator = (status == Status::Validated).then_some(actor.id);
    let change = db::content::set_status::<O>(pool, id, status, validator)
        .await?
        .ok_or(ContentRefusal::NotFound)?;
    tracing::info!(kind = ?O::KIND, %id, actor_id = %actor.id, ?status, "review status set");
    Ok(change)
}

/// The usernames of the accounts `ids` name, for those that still exist.
///
/// # Errors
///
/// Fails when the database cannot be read.
pub async fn usernames(
    exec: impl PgExecutor<'_>,
    ids: impl IntoIterator<Item = Option<Uuid>>,
) -> sqlx::Result<HashMap<Uuid, String>> {
    let mut ids: Vec<Uuid> = ids.into_iter().flatten().collect();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(db::users::usernames(exec, &ids)
        .await?
        .into_iter()
        .collect())
}
