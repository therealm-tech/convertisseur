use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{
    content::Track,
    tracks::{OutlineRow, StudyCard, StudyLesson},
};

/// The visible units and lessons of `track`, or of every track when `None`,
/// ordered by track, unit then lesson.
pub async fn outline(
    exec: impl PgExecutor<'_>,
    track: Option<Track>,
) -> sqlx::Result<Vec<OutlineRow>> {
    sqlx::query_as(include_str!("../../queries/tracks/outline.sql"))
        .bind(track)
        .fetch_all(exec)
        .await
}

pub async fn find_lesson(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    track: Track,
) -> sqlx::Result<Option<StudyLesson>> {
    sqlx::query_as(include_str!("../../queries/tracks/find_lesson.sql"))
        .bind(id)
        .bind(track)
        .fetch_optional(exec)
        .await
}

/// The visible cards of the lesson, in order.
pub async fn cards(exec: impl PgExecutor<'_>, lesson_id: Uuid) -> sqlx::Result<Vec<StudyCard>> {
    sqlx::query_as(include_str!("../../queries/tracks/cards.sql"))
        .bind(lesson_id)
        .fetch_all(exec)
        .await
}
