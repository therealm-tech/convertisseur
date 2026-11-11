use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::{
    content::Card,
    db,
    review::{CardReview, LessonFilter, LessonSummary, Totals},
};

/// One page of the lessons matching `filter`, in the order of the tracks.
pub async fn lessons(
    exec: impl PgExecutor<'_>,
    filter: &LessonFilter,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<LessonSummary>> {
    sqlx::query_as(include_str!("../../queries/review/lessons.sql"))
        .bind(filter.unit_id)
        .bind(filter.pending_review)
        .bind(filter.missing_audio)
        .bind(limit)
        .bind(offset)
        .fetch_all(exec)
        .await
}

pub async fn count_lessons(exec: impl PgExecutor<'_>, filter: &LessonFilter) -> sqlx::Result<i64> {
    sqlx::query_scalar(include_str!("../../queries/review/count_lessons.sql"))
        .bind(filter.unit_id)
        .bind(filter.pending_review)
        .bind(filter.missing_audio)
        .fetch_one(exec)
        .await
}

pub async fn totals(exec: impl PgExecutor<'_>) -> sqlx::Result<Totals> {
    sqlx::query_as(include_str!("../../queries/review/totals.sql"))
        .fetch_one(exec)
        .await
}

pub async fn lesson_exists(exec: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar(include_str!("../../queries/lessons/exists.sql"))
        .bind(id)
        .fetch_one(exec)
        .await
}

/// One page of the lesson's cards, in order, read as a single card is.
pub async fn cards_of_lesson(
    pool: &PgPool,
    lesson_id: Uuid,
    limit: i64,
    offset: i64,
) -> sqlx::Result<Vec<Card>> {
    let ids: Vec<Uuid> = sqlx::query_scalar(include_str!("../../queries/cards/ids_by_lesson.sql"))
        .bind(lesson_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    db::content::find_cards(pool, &ids).await
}

pub async fn count_cards_of_lesson(
    exec: impl PgExecutor<'_>,
    lesson_id: Uuid,
) -> sqlx::Result<i64> {
    sqlx::query_scalar(include_str!("../../queries/cards/count_by_lesson.sql"))
        .bind(lesson_id)
        .fetch_one(exec)
        .await
}

/// The cards among `ids` that belong to the lesson, locked until the end
/// of the transaction.
pub async fn lock_cards_in_lesson(
    exec: impl PgExecutor<'_>,
    lesson_id: Uuid,
    ids: &[Uuid],
) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(include_str!("../../queries/cards/lock_in_lesson.sql"))
        .bind(lesson_id)
        .bind(ids)
        .fetch_all(exec)
        .await
}

/// Validates the cards among `ids` that are to review without a review
/// note; returns how many it validated.
pub async fn validate_unflagged(
    exec: impl PgExecutor<'_>,
    lesson_id: Uuid,
    ids: &[Uuid],
    validator: Uuid,
) -> sqlx::Result<u64> {
    let result = sqlx::query(include_str!("../../queries/cards/validate_unflagged.sql"))
        .bind(lesson_id)
        .bind(ids)
        .bind(validator)
        .execute(exec)
        .await?;
    Ok(result.rows_affected())
}

pub async fn card_reviews(
    exec: impl PgExecutor<'_>,
    ids: &[Uuid],
) -> sqlx::Result<Vec<CardReview>> {
    sqlx::query_as(include_str!("../../queries/cards/statuses.sql"))
        .bind(ids)
        .fetch_all(exec)
        .await
}
