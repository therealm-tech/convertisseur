//! The review queue: lessons with the review and audio state of their cards,
//! and the validation of a lesson's cards in one go.

use std::collections::HashSet;

use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    content::{ContentRefusal, Status, Track},
    db,
    users::User,
};

/// Which lessons the queue lists; every filter left out matches all.
pub struct LessonFilter {
    pub unit_id: Option<Uuid>,
    /// Only the lessons with something to review: themselves, their unit or
    /// one of their cards.
    pub pending_review: bool,
    /// Only the lessons with a card without a recording.
    pub missing_audio: bool,
}

/// A lesson as the queue shows it, with its unit and its cards' counts. A
/// card is flagged when it is to review with a review note; the cards to
/// review count the others.
#[derive(sqlx::FromRow)]
pub struct LessonSummary {
    pub id: Uuid,
    pub position: i32,
    pub title: String,
    pub status: Status,
    pub review_note: Option<String>,
    pub has_audio: bool,
    pub unit_id: Uuid,
    pub unit_track: Track,
    pub unit_position: i32,
    pub unit_title: String,
    pub unit_status: Status,
    pub unit_review_note: Option<String>,
    pub validated_cards: i64,
    pub flagged_cards: i64,
    pub cards_to_review: i64,
    pub cards_without_audio: i64,
}

/// What the queue counts whatever its filters.
#[derive(sqlx::FromRow)]
pub struct Totals {
    /// Units, lessons and cards to review, flagged or not.
    pub items_to_review: i64,
    pub cards_without_audio: i64,
}

/// A card's review state after a validation in bulk.
#[derive(sqlx::FromRow)]
pub struct CardReview {
    pub id: Uuid,
    pub status: Status,
    pub validated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub validated_by: Option<Uuid>,
    pub review_note: Option<String>,
}

/// Validates the cards `card_ids` of the lesson that are to review without
/// a review note, and returns the state of every one of them. Flagged cards
/// stay to review, validated ones keep their validation. All or nothing:
/// when an id is not a card of the lesson, nothing is written.
pub async fn validate_cards(
    pool: &PgPool,
    actor: &User,
    lesson_id: Uuid,
    card_ids: &[Uuid],
) -> Result<Vec<CardReview>, ContentRefusal> {
    let wanted: Vec<Uuid> = card_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let mut tx = pool.begin().await?;
    db::content::lock(&mut *tx).await?;
    if !db::review::lesson_exists(&mut *tx, lesson_id).await? {
        return Err(ContentRefusal::NotFound);
    }
    let found = db::review::lock_cards_in_lesson(&mut *tx, lesson_id, &wanted).await?;
    if found.len() != wanted.len() {
        tracing::debug!(%lesson_id, asked = wanted.len(), found = found.len(), "cards not in the lesson");
        return Err(ContentRefusal::NotInParent);
    }
    let validated = db::review::validate_unflagged(&mut *tx, lesson_id, &wanted, actor.id).await?;
    let cards = db::review::card_reviews(&mut *tx, &wanted).await?;
    tx.commit().await?;
    tracing::info!(
        %lesson_id, actor_id = %actor.id, asked = wanted.len(), validated,
        "cards validated in bulk"
    );
    Ok(cards)
}
