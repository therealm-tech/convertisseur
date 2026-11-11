use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{
    content::Track,
    progress::{ExerciseCardRow, ExerciseResult, LessonViews},
};

/// The visible cards of `track`, or of every track when `None`.
pub async fn exercise_cards(
    exec: impl PgExecutor<'_>,
    track: Option<Track>,
) -> sqlx::Result<Vec<ExerciseCardRow>> {
    sqlx::query_as(include_str!("../../queries/progress/exercise_cards.sql"))
        .bind(track)
        .fetch_all(exec)
        .await
}

pub async fn lesson_views(
    exec: impl PgExecutor<'_>,
    user_id: Uuid,
) -> sqlx::Result<Vec<LessonViews>> {
    sqlx::query_as(include_str!("../../queries/progress/lesson_views.sql"))
        .bind(user_id)
        .fetch_all(exec)
        .await
}

pub async fn exercise_results(
    exec: impl PgExecutor<'_>,
    user_id: Uuid,
) -> sqlx::Result<Vec<ExerciseResult>> {
    sqlx::query_as(include_str!("../../queries/progress/exercise_results.sql"))
        .bind(user_id)
        .fetch_all(exec)
        .await
}

/// The cards of the lesson the account has seen, visible or not.
pub async fn seen_cards(
    exec: impl PgExecutor<'_>,
    user_id: Uuid,
    lesson_id: Uuid,
) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(include_str!("../../queries/progress/seen_cards.sql"))
        .bind(user_id)
        .bind(lesson_id)
        .fetch_all(exec)
        .await
}

/// The lesson and track of a card learners see.
pub async fn find_card(
    exec: impl PgExecutor<'_>,
    card_id: Uuid,
) -> sqlx::Result<Option<(Uuid, Track)>> {
    sqlx::query_as(include_str!("../../queries/progress/find_card.sql"))
        .bind(card_id)
        .fetch_optional(exec)
        .await
}

/// The track of a lesson learners see.
pub async fn find_lesson(
    exec: impl PgExecutor<'_>,
    lesson_id: Uuid,
) -> sqlx::Result<Option<Track>> {
    sqlx::query_scalar(include_str!("../../queries/progress/find_lesson.sql"))
        .bind(lesson_id)
        .fetch_optional(exec)
        .await
}

pub async fn record_view(
    exec: impl PgExecutor<'_>,
    user_id: Uuid,
    card_id: Uuid,
) -> sqlx::Result<()> {
    sqlx::query(include_str!("../../queries/progress/record_view.sql"))
        .bind(user_id)
        .bind(card_id)
        .execute(exec)
        .await
        .map(|_| ())
}

/// Folds an attempt into the account's result for the lesson, and returns
/// the result.
pub async fn record_attempt(
    exec: impl PgExecutor<'_>,
    user_id: Uuid,
    lesson_id: Uuid,
    first_try: i32,
    total: i32,
    passed: bool,
) -> sqlx::Result<ExerciseResult> {
    sqlx::query_as(include_str!("../../queries/progress/record_attempt.sql"))
        .bind(user_id)
        .bind(lesson_id)
        .bind(first_try)
        .bind(total)
        .bind(passed)
        .fetch_one(exec)
        .await
}
