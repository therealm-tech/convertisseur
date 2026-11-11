//! What an account has done in the lessons it sees, and which of them stay
//! locked. A track is one path, its units and lessons in order: a lesson
//! opens once the one before it is open and its exercise passed, or has no
//! exercise. A lesson the account has started never locks again, so that a
//! lesson added or validated upstream does not take back what it reached.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    content::Track,
    db,
    exercise::{self, ExerciseCard},
    tracks::{self, Outline},
};

/// A visible card, as far as the exercise is concerned.
#[derive(sqlx::FromRow)]
pub struct ExerciseCardRow {
    pub lesson_id: Uuid,
    pub arabic: String,
    pub meaning_fr: Option<String>,
    pub has_forms: bool,
    pub has_audio: bool,
}

/// The cards an account has seen in one lesson.
#[derive(sqlx::FromRow)]
pub struct LessonViews {
    pub lesson_id: Uuid,
    /// Among the cards learners see.
    pub seen_card_count: i64,
    pub last_seen_at: Option<DateTime<Utc>>,
}

/// An account's best exercise result in a lesson.
#[derive(Clone, sqlx::FromRow)]
pub struct ExerciseResult {
    pub lesson_id: Uuid,
    pub best_first_try: i32,
    pub best_total: i32,
    pub best_at: DateTime<Utc>,
    pub passed_at: Option<DateTime<Utc>>,
    pub last_attempt_at: DateTime<Utc>,
}

/// Where an account stands in one visible lesson.
#[derive(Clone)]
pub struct LessonState {
    pub has_exercise: bool,
    pub locked: bool,
    pub seen_card_count: i64,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub exercise: Option<ExerciseResult>,
}

/// The visible tracks, and the account's state in each of their lessons.
pub struct Progress {
    pub outlines: Vec<Outline>,
    pub lessons: HashMap<Uuid, LessonState>,
}

impl Progress {
    /// The lesson after `lesson_id` on its track's path, when there is one.
    #[must_use]
    pub fn next_lesson(&self, lesson_id: Uuid) -> Option<&tracks::OutlineLesson> {
        self.outlines.iter().find_map(|outline| {
            let mut path = outline.units.iter().flat_map(|unit| &unit.lessons);
            path.by_ref().find(|lesson| lesson.id == lesson_id)?;
            path.next()
        })
    }
}

/// The account's progress in `tracks`, read in a handful of queries.
///
/// # Errors
///
/// Fails when the database does.
pub async fn load(pool: &PgPool, user_id: Uuid, tracks: &[Track]) -> sqlx::Result<Progress> {
    let only = match tracks {
        [track] => Some(*track),
        _ => None,
    };
    let rows = db::tracks::outline(pool, only).await?;
    let outlines = tracks::outlines(tracks, rows);
    let cards = db::progress::exercise_cards(pool, only).await?;
    let views = db::progress::lesson_views(pool, user_id).await?;
    let results = db::progress::exercise_results(pool, user_id).await?;
    let lessons = states(&outlines, &cards, views, results);
    Ok(Progress { outlines, lessons })
}

/// Each visible lesson's state, given its cards and the account's views and
/// results.
#[must_use]
pub fn states(
    outlines: &[Outline],
    cards: &[ExerciseCardRow],
    views: Vec<LessonViews>,
    results: Vec<ExerciseResult>,
) -> HashMap<Uuid, LessonState> {
    let mut cards_by_lesson: HashMap<Uuid, Vec<ExerciseCard<'_>>> = HashMap::new();
    for card in cards {
        cards_by_lesson
            .entry(card.lesson_id)
            .or_default()
            .push(ExerciseCard {
                arabic: &card.arabic,
                meaning: card.meaning_fr.as_deref(),
                has_forms: card.has_forms,
                has_audio: card.has_audio,
            });
    }
    let mut views: HashMap<Uuid, LessonViews> =
        views.into_iter().map(|v| (v.lesson_id, v)).collect();
    let mut results: HashMap<Uuid, ExerciseResult> =
        results.into_iter().map(|r| (r.lesson_id, r)).collect();

    let mut states = HashMap::new();
    for outline in outlines {
        // Whether the lesson before is open and lets the next one through.
        let mut way_open = true;
        for lesson in outline.units.iter().flat_map(|unit| &unit.lessons) {
            let has_exercise = cards_by_lesson
                .get(&lesson.id)
                .is_some_and(|cards| exercise::has_exercise(cards));
            let view = views.remove(&lesson.id);
            let result = results.remove(&lesson.id);
            let started = view.is_some() || result.is_some();
            let passed = result.as_ref().is_some_and(|r| r.passed_at.is_some());
            let locked = !(way_open || started);
            way_open = !locked && (passed || !has_exercise);
            states.insert(
                lesson.id,
                LessonState {
                    has_exercise,
                    locked,
                    seen_card_count: view.as_ref().map_or(0, |v| v.seen_card_count),
                    last_seen_at: view.and_then(|v| v.last_seen_at),
                    exercise: result,
                },
            );
        }
    }
    states
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::{ExerciseCardRow, ExerciseResult, LessonViews, states};
    use crate::{
        content::Track,
        tracks::{Outline, OutlineLesson, OutlineUnit},
    };

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    /// One unit per slice, each lesson numbered from 1 across the track.
    fn track(units: &[&[u128]]) -> Vec<Outline> {
        vec![Outline {
            track: Track::Arabe,
            units: units
                .iter()
                .enumerate()
                .map(|(index, lessons)| OutlineUnit {
                    id: id(1000 + index as u128),
                    title: format!("unit {index}"),
                    lessons: lessons
                        .iter()
                        .map(|&n| OutlineLesson {
                            id: id(n),
                            title: format!("lesson {n}"),
                            card_count: 3,
                        })
                        .collect(),
                })
                .collect(),
        }]
    }

    /// Three cards that make an exercise, for each lesson named.
    fn with_exercise(lessons: &[u128]) -> Vec<ExerciseCardRow> {
        lessons
            .iter()
            .flat_map(|&n| {
                ["un", "deux", "trois"].map(|meaning| ExerciseCardRow {
                    lesson_id: id(n),
                    arabic: format!("{meaning} {n}"),
                    meaning_fr: Some(meaning.to_owned()),
                    has_forms: false,
                    has_audio: false,
                })
            })
            .collect()
    }

    fn result(n: u128, passed: bool) -> ExerciseResult {
        let now = Utc::now();
        ExerciseResult {
            lesson_id: id(n),
            best_first_try: if passed { 12 } else { 6 },
            best_total: 12,
            best_at: now,
            passed_at: passed.then_some(now),
            last_attempt_at: now,
        }
    }

    fn seen(n: u128) -> LessonViews {
        LessonViews {
            lesson_id: id(n),
            seen_card_count: 1,
            last_seen_at: Some(Utc::now()),
        }
    }

    fn locked(
        outlines: &[Outline],
        cards: &[ExerciseCardRow],
        views: Vec<LessonViews>,
        results: Vec<ExerciseResult>,
    ) -> Vec<bool> {
        let states = states(outlines, cards, views, results);
        outlines[0]
            .units
            .iter()
            .flat_map(|unit| &unit.lessons)
            .map(|lesson| states[&lesson.id].locked)
            .collect()
    }

    #[test]
    fn opens_only_the_first_lesson_of_a_new_account() {
        let outlines = track(&[&[1, 2], &[3]]);
        let cards = with_exercise(&[1, 2, 3]);

        assert_eq!(
            locked(&outlines, &cards, vec![], vec![]),
            [false, true, true]
        );
    }

    #[test]
    fn opens_the_next_lesson_once_the_exercise_is_passed_across_units() {
        let outlines = track(&[&[1, 2], &[3]]);
        let cards = with_exercise(&[1, 2, 3]);

        assert_eq!(
            locked(&outlines, &cards, vec![], vec![result(1, true)]),
            [false, false, true]
        );
        assert_eq!(
            locked(
                &outlines,
                &cards,
                vec![],
                vec![result(1, true), result(2, true)]
            ),
            [false, false, false]
        );
    }

    #[test]
    fn keeps_the_next_lesson_locked_after_an_exercise_not_passed() {
        let outlines = track(&[&[1, 2]]);
        let cards = with_exercise(&[1, 2]);

        assert_eq!(
            locked(&outlines, &cards, vec![seen(1)], vec![result(1, false)]),
            [false, true]
        );
    }

    #[test]
    fn lets_a_lesson_without_exercise_through_only_once_it_is_open() {
        let outlines = track(&[&[1, 2, 3, 4]]);
        let cards = with_exercise(&[1, 3, 4]);

        assert_eq!(
            locked(&outlines, &cards, vec![], vec![]),
            [false, true, true, true]
        );
        assert_eq!(
            locked(&outlines, &cards, vec![], vec![result(1, true)]),
            [false, false, false, true]
        );
    }

    #[test]
    fn never_locks_a_lesson_already_started() {
        let outlines = track(&[&[1, 5, 2, 3]]);
        let cards = with_exercise(&[1, 5, 2, 3]);

        assert_eq!(
            locked(
                &outlines,
                &cards,
                vec![seen(2)],
                vec![result(1, true), result(2, true)]
            ),
            [false, false, false, false]
        );
        assert_eq!(
            locked(&outlines, &cards, vec![seen(2)], vec![result(1, false)]),
            [false, true, false, true]
        );
    }

    #[test]
    fn counts_what_the_account_has_seen_and_its_result() {
        let outlines = track(&[&[1]]);
        let states = states(
            &outlines,
            &with_exercise(&[1]),
            vec![seen(1)],
            vec![result(1, true)],
        );

        let state = &states[&id(1)];
        assert!(state.has_exercise);
        assert_eq!(state.seen_card_count, 1);
        assert!(state.last_seen_at.is_some());
        assert_eq!(state.exercise.as_ref().unwrap().best_first_try, 12);
    }
}
