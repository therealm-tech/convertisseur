use sqlx::{PgExecutor, Postgres};
use uuid::Uuid;

use crate::{
    content::{
        Card, CardFields, ContentItem, ContentKind, ContentQuery, Lesson, LessonFields, Status,
        StatusChange, Track, Unit, UnitFields,
    },
    letters::LetterForms,
};

/// Key of the transaction-level advisory lock that serialises every write to
/// units, lessons and cards, so that positions stay dense and unique.
const CONTENT_LOCK_KEY: i64 = 0x636f_6e76_636f_6e74;

pub async fn lock(exec: impl PgExecutor<'_>) -> sqlx::Result<()> {
    sqlx::query(include_str!("../../queries/content/lock.sql"))
        .bind(CONTENT_LOCK_KEY)
        .execute(exec)
        .await?;
    Ok(())
}

/// A content table whose rows are ordered among siblings sharing a parent.
pub trait Ordered: Send {
    /// What groups siblings: the track of a unit, the unit of a lesson, the
    /// lesson of a card.
    type Parent: for<'q> sqlx::Encode<'q, Postgres>
        + for<'r> sqlx::Decode<'r, Postgres>
        + sqlx::Type<Postgres>
        + Copy
        + PartialEq
        + Send
        + Sync
        + Unpin;

    const KIND: ContentKind;
    const COUNT_SIBLINGS: &'static str;
    const SHIFT: &'static str;
    const LOCK: &'static str;
    const DELETE: &'static str;
    /// Whether a parent row exists; `None` when the parent is not a row.
    const PARENT_EXISTS: Option<&'static str>;
    const SET_STATUS: &'static str;
    const SET_REVIEW_NOTE: &'static str;
    /// Whether rows of the next level hang off this one; `None` for cards.
    const HAS_CHILDREN: Option<&'static str>;
}

pub struct Units;

impl Ordered for Units {
    type Parent = Track;
    const KIND: ContentKind = ContentKind::Unit;
    const COUNT_SIBLINGS: &'static str = include_str!("../../queries/units/count_siblings.sql");
    const SHIFT: &'static str = include_str!("../../queries/units/shift.sql");
    const LOCK: &'static str = include_str!("../../queries/units/lock.sql");
    const DELETE: &'static str = include_str!("../../queries/units/delete.sql");
    const PARENT_EXISTS: Option<&'static str> = None;
    const SET_STATUS: &'static str = include_str!("../../queries/units/set_status.sql");
    const SET_REVIEW_NOTE: &'static str = include_str!("../../queries/units/set_review_note.sql");
    const HAS_CHILDREN: Option<&'static str> =
        Some(include_str!("../../queries/units/has_children.sql"));
}

pub struct Lessons;

impl Ordered for Lessons {
    type Parent = Uuid;
    const KIND: ContentKind = ContentKind::Lesson;
    const COUNT_SIBLINGS: &'static str = include_str!("../../queries/lessons/count_siblings.sql");
    const SHIFT: &'static str = include_str!("../../queries/lessons/shift.sql");
    const LOCK: &'static str = include_str!("../../queries/lessons/lock.sql");
    const DELETE: &'static str = include_str!("../../queries/lessons/delete.sql");
    const PARENT_EXISTS: Option<&'static str> =
        Some(include_str!("../../queries/units/exists.sql"));
    const SET_STATUS: &'static str = include_str!("../../queries/lessons/set_status.sql");
    const SET_REVIEW_NOTE: &'static str = include_str!("../../queries/lessons/set_review_note.sql");
    const HAS_CHILDREN: Option<&'static str> =
        Some(include_str!("../../queries/lessons/has_children.sql"));
}

pub struct Cards;

/// A content table whose rows can hold a recording.
pub trait WithAudio: Ordered {
    const LOCK_AUDIO: &'static str;
    const SET_AUDIO: &'static str;
}

impl WithAudio for Lessons {
    const LOCK_AUDIO: &'static str = include_str!("../../queries/lessons/lock_audio.sql");
    const SET_AUDIO: &'static str = include_str!("../../queries/lessons/set_audio.sql");
}

impl WithAudio for Cards {
    const LOCK_AUDIO: &'static str = include_str!("../../queries/cards/lock_audio.sql");
    const SET_AUDIO: &'static str = include_str!("../../queries/cards/set_audio.sql");
}

impl Ordered for Cards {
    type Parent = Uuid;
    const KIND: ContentKind = ContentKind::Card;
    const COUNT_SIBLINGS: &'static str = include_str!("../../queries/cards/count_siblings.sql");
    const SHIFT: &'static str = include_str!("../../queries/cards/shift.sql");
    const LOCK: &'static str = include_str!("../../queries/cards/lock.sql");
    const DELETE: &'static str = include_str!("../../queries/cards/delete.sql");
    const PARENT_EXISTS: Option<&'static str> =
        Some(include_str!("../../queries/lessons/exists.sql"));
    const SET_STATUS: &'static str = include_str!("../../queries/cards/set_status.sql");
    const SET_REVIEW_NOTE: &'static str = include_str!("../../queries/cards/set_review_note.sql");
    const HAS_CHILDREN: Option<&'static str> = None;
}

/// Where a row sits among its siblings.
#[derive(sqlx::FromRow)]
pub struct Placement<P> {
    pub parent: P,
    pub position: i32,
    /// Whether the row comes from the content files.
    pub imported: bool,
}

/// How many rows share `parent`, `except` aside.
pub async fn count_siblings<O: Ordered>(
    exec: impl PgExecutor<'_>,
    parent: O::Parent,
    except: Option<Uuid>,
) -> sqlx::Result<i64> {
    sqlx::query_scalar(O::COUNT_SIBLINGS)
        .bind(parent)
        .bind(except)
        .fetch_one(exec)
        .await
}

/// Moves by `delta` places the siblings under `parent` from position `from`
/// on, `except` aside.
pub async fn shift<O: Ordered>(
    exec: impl PgExecutor<'_>,
    parent: O::Parent,
    from: i32,
    delta: i32,
    except: Option<Uuid>,
) -> sqlx::Result<()> {
    sqlx::query(O::SHIFT)
        .bind(parent)
        .bind(from)
        .bind(delta)
        .bind(except)
        .execute(exec)
        .await?;
    Ok(())
}

/// The row's placement, locked until the end of the transaction.
pub async fn lock_row<O: Ordered>(
    exec: impl PgExecutor<'_>,
    id: Uuid,
) -> sqlx::Result<Option<Placement<O::Parent>>> {
    sqlx::query_as(O::LOCK).bind(id).fetch_optional(exec).await
}

/// A deleted row: where it sat, and the recording it held.
#[derive(sqlx::FromRow)]
pub struct Deleted<P> {
    pub parent: P,
    pub position: i32,
    pub audio_id: Option<Uuid>,
}

/// Deletes the row; returns what it was, or `None` when it did not exist.
pub async fn delete<O: Ordered>(
    exec: impl PgExecutor<'_>,
    id: Uuid,
) -> sqlx::Result<Option<Deleted<O::Parent>>> {
    sqlx::query_as(O::DELETE)
        .bind(id)
        .fetch_optional(exec)
        .await
}

pub async fn parent_exists<O: Ordered>(
    exec: impl PgExecutor<'_>,
    parent: O::Parent,
) -> sqlx::Result<bool> {
    match O::PARENT_EXISTS {
        Some(query) => sqlx::query_scalar(query).bind(parent).fetch_one(exec).await,
        None => Ok(true),
    }
}

pub async fn has_children<O: Ordered>(exec: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<bool> {
    match O::HAS_CHILDREN {
        Some(query) => sqlx::query_scalar(query).bind(id).fetch_one(exec).await,
        None => Ok(false),
    }
}

/// Sets the review status; a validation records `validator`. Returns `None`
/// when the row does not exist.
pub async fn set_status<O: Ordered>(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    status: Status,
    validator: Option<Uuid>,
) -> sqlx::Result<Option<StatusChange>> {
    sqlx::query_as(O::SET_STATUS)
        .bind(id)
        .bind(status)
        .bind(validator)
        .fetch_optional(exec)
        .await
}

/// Sets or clears the review note; returns the stored note, or `None` in
/// the outer option when the row does not exist.
pub async fn set_review_note<O: Ordered>(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    note: Option<&str>,
) -> sqlx::Result<Option<Option<String>>> {
    sqlx::query_scalar(O::SET_REVIEW_NOTE)
        .bind(id)
        .bind(note)
        .fetch_optional(exec)
        .await
}

/// The recording the row holds, the row locked until the end of the
/// transaction; `None` when the row does not exist.
pub async fn lock_audio<O: WithAudio>(
    exec: impl PgExecutor<'_>,
    id: Uuid,
) -> sqlx::Result<Option<Option<Uuid>>> {
    sqlx::query_scalar(O::LOCK_AUDIO)
        .bind(id)
        .fetch_optional(exec)
        .await
}

/// Points the row at another recording, or none, which sends it back to
/// review.
pub async fn set_audio<O: WithAudio>(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    audio_id: Option<Uuid>,
    editor: Uuid,
) -> sqlx::Result<()> {
    sqlx::query(O::SET_AUDIO)
        .bind(id)
        .bind(audio_id)
        .bind(editor)
        .execute(exec)
        .await?;
    Ok(())
}

/// Inserts the row at `position`; `import_key` names the content file entry
/// it comes from, `None` for what staff write.
pub async fn insert_unit(
    exec: impl PgExecutor<'_>,
    fields: &UnitFields,
    position: i32,
    import_key: Option<&str>,
    author: Uuid,
) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(include_str!("../../queries/units/insert.sql"))
        .bind(fields.track)
        .bind(position)
        .bind(&fields.title)
        .bind(import_key)
        .bind(author)
        .fetch_one(exec)
        .await
}

/// Writes the fields and position; when a field other than the place
/// changes, the unit goes back to review. Returns whether anything changed.
pub async fn update_unit(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    fields: &UnitFields,
    position: i32,
    editor: Uuid,
) -> sqlx::Result<bool> {
    let result = sqlx::query(include_str!("../../queries/units/update.sql"))
        .bind(id)
        .bind(fields.track)
        .bind(position)
        .bind(&fields.title)
        .bind(editor)
        .execute(exec)
        .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn find_unit(exec: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<Option<Unit>> {
    sqlx::query_as(include_str!("../../queries/units/find.sql"))
        .bind(id)
        .fetch_optional(exec)
        .await
}

/// Inserts the row at `position`; `import_key` names the content file entry
/// it comes from, `None` for what staff write.
pub async fn insert_lesson(
    exec: impl PgExecutor<'_>,
    fields: &LessonFields,
    position: i32,
    import_key: Option<&str>,
    author: Uuid,
) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(include_str!("../../queries/lessons/insert.sql"))
        .bind(fields.unit_id)
        .bind(position)
        .bind(&fields.title)
        .bind(import_key)
        .bind(author)
        .fetch_one(exec)
        .await
}

/// Writes the fields and position; when a field other than the place
/// changes, the lesson goes back to review. Returns whether anything changed.
pub async fn update_lesson(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    fields: &LessonFields,
    position: i32,
    editor: Uuid,
) -> sqlx::Result<bool> {
    let result = sqlx::query(include_str!("../../queries/lessons/update.sql"))
        .bind(id)
        .bind(fields.unit_id)
        .bind(position)
        .bind(&fields.title)
        .bind(editor)
        .execute(exec)
        .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn find_lesson(exec: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<Option<Lesson>> {
    sqlx::query_as(include_str!("../../queries/lessons/find.sql"))
        .bind(id)
        .fetch_optional(exec)
        .await
}

/// Inserts the row at `position`; `import_key` names the content file entry
/// it comes from, `None` for what staff write, and `forms` is only ever
/// written by the content files.
pub async fn insert_card(
    exec: impl PgExecutor<'_>,
    fields: &CardFields,
    forms: Option<&LetterForms>,
    position: i32,
    import_key: Option<&str>,
    author: Uuid,
) -> sqlx::Result<Uuid> {
    let [initial, medial, last] = LetterForms::columns(forms);
    sqlx::query_scalar(include_str!("../../queries/cards/insert.sql"))
        .bind(fields.lesson_id)
        .bind(position)
        .bind(&fields.arabic)
        .bind(&fields.transliteration)
        .bind(&fields.meaning_fr)
        .bind(fields.variety)
        .bind(initial)
        .bind(medial)
        .bind(last)
        .bind(import_key)
        .bind(author)
        .fetch_one(exec)
        .await
}

/// Writes the fields and position; when a field other than the place
/// changes, the card goes back to review. Returns whether anything changed.
pub async fn update_card(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    fields: &CardFields,
    position: i32,
    editor: Uuid,
) -> sqlx::Result<bool> {
    let result = sqlx::query(include_str!("../../queries/cards/update.sql"))
        .bind(id)
        .bind(fields.lesson_id)
        .bind(position)
        .bind(&fields.arabic)
        .bind(&fields.transliteration)
        .bind(&fields.meaning_fr)
        .bind(fields.variety)
        .bind(editor)
        .execute(exec)
        .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn find_card(exec: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<Option<Card>> {
    sqlx::query_as(include_str!("../../queries/cards/find.sql"))
        .bind([id])
        .fetch_optional(exec)
        .await
}

/// The cards among `ids` that exist, in their lessons' order.
pub async fn find_cards(exec: impl PgExecutor<'_>, ids: &[Uuid]) -> sqlx::Result<Vec<Card>> {
    sqlx::query_as(include_str!("../../queries/cards/find.sql"))
        .bind(ids)
        .fetch_all(exec)
        .await
}

/// One page of the units, lessons and cards matching `query`, the most
/// recently edited first.
pub async fn list(
    exec: impl PgExecutor<'_>,
    query: &ContentQuery<'_>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<ContentItem>> {
    sqlx::query_as(include_str!("../../queries/content/list.sql"))
        .bind(query.status)
        .bind(query.track)
        .bind(query.kind)
        .bind(query.text)
        .bind(query.has_review_note)
        .bind(limit)
        .bind(offset)
        .fetch_all(exec)
        .await
}

/// How many items match `query`'s track, kind and text, per review status
/// and per presence of a review note: `(status, noted, items)`.
pub async fn counts(
    exec: impl PgExecutor<'_>,
    query: &ContentQuery<'_>,
) -> sqlx::Result<Vec<(Status, bool, i64)>> {
    sqlx::query_as(include_str!("../../queries/content/counts.sql"))
        .bind(query.track)
        .bind(query.kind)
        .bind(query.text)
        .fetch_all(exec)
        .await
}
