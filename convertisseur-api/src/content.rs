//! Learning content: units, lessons and cards, their order among their
//! siblings and their review. Every write runs in one transaction under the
//! content lock.

use std::collections::HashMap;

use anyhow::Context;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgExecutor, PgPool};
use uuid::Uuid;

use crate::{
    audio,
    db::{
        self,
        audio::NewAudio,
        content::{Ordered, WithAudio},
    },
    letters::FormColumns,
    storage::{ObjectStore, Storage},
    users::User,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, sqlx::Type)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, sqlx::Type, serde::Deserialize)]
#[sqlx(type_name = "arabic_variety", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
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
    /// The content file entry the row comes from; `None` for what staff
    /// write in the admin area.
    pub import_key: Option<String>,
    #[sqlx(rename = "review_note")]
    pub note: Option<String>,
}

impl Review {
    /// The accounts the review names.
    #[must_use]
    pub fn authors(&self) -> [Option<Uuid>; 3] {
        [self.created_by, self.updated_by, self.validated_by]
    }
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
    pub audio: AudioColumns,
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
    #[sqlx(flatten)]
    pub forms: FormColumns,
    #[sqlx(flatten)]
    pub audio: AudioColumns,
    #[sqlx(flatten)]
    pub review: Review,
}

/// A stored recording.
#[derive(sqlx::FromRow)]
pub struct Audio {
    pub id: Uuid,
    pub content_type: String,
    pub size_bytes: i64,
    pub duration_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// The recording a card or lesson holds, all `NULL` when it holds none.
#[derive(sqlx::FromRow)]
pub struct AudioColumns {
    #[sqlx(rename = "audio_id")]
    pub id: Option<Uuid>,
    #[sqlx(rename = "audio_content_type")]
    pub content_type: Option<String>,
    #[sqlx(rename = "audio_size_bytes")]
    pub size_bytes: Option<i64>,
    #[sqlx(rename = "audio_duration_ms")]
    pub duration_ms: Option<i32>,
    #[sqlx(rename = "audio_created_at")]
    pub created_at: Option<DateTime<Utc>>,
}

impl AudioColumns {
    #[must_use]
    pub fn into_audio(self) -> Option<Audio> {
        Some(Audio {
            id: self.id?,
            content_type: self.content_type?,
            size_bytes: self.size_bytes?,
            duration_ms: self.duration_ms,
            created_at: self.created_at?,
        })
    }
}

/// A recording as playback needs it, and whether learners see the card or
/// lesson holding it: validated, with every row above it.
#[derive(sqlx::FromRow)]
pub struct Playable {
    pub object_key: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub visible: bool,
}

/// A recording to attach, already sniffed.
pub struct Upload {
    pub format: audio::Format,
    pub bytes: Bytes,
    /// The length the client measured; used only when the file does not
    /// give its own.
    pub claimed_duration_ms: Option<u32>,
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
    /// Whether the item carries a review note.
    pub has_review_note: Option<bool>,
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
    pub import_key: Option<String>,
    pub review_note: Option<String>,
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
    /// The row comes from the content files, which alone change its text and
    /// place, and delete it.
    Imported,
    /// A row the request names is not a child of the row it names.
    NotInParent,
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
pub(crate) async fn place<O: Ordered>(
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
/// it is given, and returns whether anything changed. Only a change of content
/// sends the row back to review; a move keeps its status.
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
    if current.imported {
        tracing::debug!(kind = ?O::KIND, %id, "update refused: imported");
        return Err(ContentRefusal::Imported);
    }
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
        tracing::info!(kind = ?O::KIND, %id, actor_id = %actor.id, position, "content updated");
    } else {
        tracing::debug!(kind = ?O::KIND, %id, actor_id = %actor.id, "content unchanged");
    }
    Ok(())
}

/// Deletes a row, its recording with it, and closes the gap it leaves;
/// succeeds when it is already gone. An imported row, a unit with lessons
/// and a lesson with cards stay.
pub async fn delete<O: Ordered>(
    pool: &PgPool,
    storage: &Storage,
    actor: &User,
    id: Uuid,
) -> Result<(), ContentRefusal> {
    let mut tx = pool.begin().await?;
    db::content::lock(&mut *tx).await?;
    let Some(current) = db::content::lock_row::<O>(&mut *tx, id).await? else {
        tracing::debug!(kind = ?O::KIND, %id, "already deleted");
        return Ok(());
    };
    if current.imported {
        tracing::debug!(kind = ?O::KIND, %id, "deletion refused: imported");
        return Err(ContentRefusal::Imported);
    }
    if db::content::has_children::<O>(&mut *tx, id).await? {
        tracing::debug!(kind = ?O::KIND, %id, "deletion refused: not empty");
        return Err(ContentRefusal::NotEmpty);
    }
    let Some(deleted) = db::content::delete::<O>(&mut *tx, id).await? else {
        tracing::debug!(kind = ?O::KIND, %id, "already deleted");
        return Ok(());
    };
    db::content::shift::<O>(
        &mut *tx,
        deleted.parent,
        deleted.position.saturating_add(1),
        -1,
        None,
    )
    .await?;
    let key = match deleted.audio_id {
        Some(audio_id) => db::audio::delete(&mut *tx, audio_id).await?,
        None => None,
    };
    tx.commit().await?;
    tracing::info!(kind = ?O::KIND, %id, actor_id = %actor.id, "content deleted");
    if let Some(key) = key {
        delete_object(storage, &key).await;
    }
    Ok(())
}

/// Deletes an object whose row is gone. A failure leaves an orphan, which
/// takes no space anyone reads: it is logged, not raised.
pub(crate) async fn delete_object(storage: &Storage, key: &str) {
    if let Err(error) = storage.delete(key).await {
        tracing::warn!(
            key,
            error = format!("{error:#}"),
            "orphaned object left in the store"
        );
    }
}

/// Stores a recording and attaches it to the row in place of the one it
/// had, which is deleted, row and object. The row goes back to review.
///
/// The object is written before the row that points at it, and the old
/// object deleted after the row stops pointing at it, so that a failure
/// leaves at worst an orphaned object, never a row without its object.
pub async fn attach_audio<O: WithAudio>(
    pool: &PgPool,
    storage: &Storage,
    actor: &User,
    id: Uuid,
    upload: Upload,
) -> Result<Audio, ContentRefusal> {
    if db::content::lock_audio::<O>(pool, id).await?.is_none() {
        return Err(ContentRefusal::NotFound);
    }
    let audio_id = Uuid::new_v4();
    let key = format!("audio/{audio_id}");
    let content_type = upload.format.content_type();
    let size = upload.bytes.len();
    let duration_ms = upload
        .format
        .duration_ms(&upload.bytes)
        .or(upload.claimed_duration_ms)
        .and_then(|ms| i32::try_from(ms).ok());
    storage
        .put(&key, content_type, upload.bytes)
        .await
        .context("failed to store the recording")?;

    let attached = async {
        let mut tx = pool.begin().await?;
        db::content::lock(&mut *tx).await?;
        let Some(previous) = db::content::lock_audio::<O>(&mut *tx, id).await? else {
            return Err(ContentRefusal::NotFound);
        };
        let audio = db::audio::insert(
            &mut *tx,
            &NewAudio {
                id: audio_id,
                object_key: &key,
                content_type,
                size_bytes: i64::try_from(size).unwrap_or(i64::MAX),
                duration_ms,
                created_by: actor.id,
            },
        )
        .await?;
        db::content::set_audio::<O>(&mut *tx, id, Some(audio_id), actor.id).await?;
        let replaced = match previous {
            Some(previous) => db::audio::delete(&mut *tx, previous).await?,
            None => None,
        };
        tx.commit().await?;
        Ok((audio, replaced))
    }
    .await;

    match attached {
        Ok((audio, replaced)) => {
            tracing::info!(
                kind = ?O::KIND, %id, actor_id = %actor.id, %audio_id, content_type, size,
                "recording attached, back to review"
            );
            if let Some(replaced) = replaced {
                delete_object(storage, &replaced).await;
            }
            Ok(audio)
        }
        Err(refusal) => {
            delete_object(storage, &key).await;
            Err(refusal)
        }
    }
}

/// Removes the row's recording, row and object; the row goes back to
/// review. Succeeds when it has none.
pub async fn detach_audio<O: WithAudio>(
    pool: &PgPool,
    storage: &Storage,
    actor: &User,
    id: Uuid,
) -> Result<(), ContentRefusal> {
    let mut tx = pool.begin().await?;
    db::content::lock(&mut *tx).await?;
    let Some(previous) = db::content::lock_audio::<O>(&mut *tx, id).await? else {
        return Err(ContentRefusal::NotFound);
    };
    let Some(previous) = previous else {
        tracing::debug!(kind = ?O::KIND, %id, "no recording to remove");
        return Ok(());
    };
    db::content::set_audio::<O>(&mut *tx, id, None, actor.id).await?;
    let key = db::audio::delete(&mut *tx, previous).await?;
    tx.commit().await?;
    tracing::info!(kind = ?O::KIND, %id, actor_id = %actor.id, "recording removed, back to review");
    if let Some(key) = key {
        delete_object(storage, &key).await;
    }
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

/// Writes the row's review note, or clears it with `None`. The note
/// leaves the status as it is: sending the row back to review is a
/// decision of its own.
pub async fn set_review_note<O: Ordered>(
    pool: &PgPool,
    actor: &User,
    id: Uuid,
    note: Option<String>,
) -> Result<Option<String>, ContentRefusal> {
    let chars = note.as_deref().map_or(0, |note| note.chars().count());
    let note = db::content::set_review_note::<O>(pool, id, note.as_deref())
        .await?
        .ok_or(ContentRefusal::NotFound)?;
    tracing::info!(kind = ?O::KIND, %id, actor_id = %actor.id, chars, "review note set");
    Ok(note)
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
