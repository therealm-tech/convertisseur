//! Whether a lesson has an exercise, and what passes one. The webapp builds
//! and grades the exercise in `src/lib/quiz.ts`; the server repeats these two
//! rules because it locks the lessons that follow an exercise not passed.
//! The shared cases in `convertisseur-webapp/src/__tests__/exerciseCases.json`
//! keep the two implementations in step.

use std::collections::HashSet;

/// The share of questions to answer right the first time, in percent.
pub const PASS_PERCENT: i64 = 90;

/// What of a card decides the questions it can ask.
pub struct ExerciseCard<'a> {
    pub arabic: &'a str,
    pub meaning: Option<&'a str>,
    pub has_forms: bool,
    pub has_audio: bool,
}

/// Whether `first_try` of `total` questions answered right the first time
/// pass the exercise.
#[must_use]
pub fn passes(first_try: i64, total: i64) -> bool {
    total > 0 && first_try * 100 >= PASS_PERCENT * total
}

/// Whether the cards make at least one question: a question needs its
/// answer and two distinct distractors drawn from the other cards.
#[must_use]
pub fn has_exercise(cards: &[ExerciseCard<'_>]) -> bool {
    cards.iter().enumerate().any(|(index, card)| {
        let others = || {
            cards
                .iter()
                .enumerate()
                .filter(move |(other, _)| *other != index)
                .map(|(_, other)| other)
        };
        let asks_meaning = card.meaning.is_some_and(|meaning| {
            enough_choices(
                meaning,
                others()
                    .filter(|other| other.arabic != card.arabic)
                    .filter_map(|other| other.meaning),
            ) || enough_choices(
                card.arabic,
                others()
                    .filter(|other| other.meaning != Some(meaning))
                    .map(|other| other.arabic),
            )
        });
        let asks_form = card.has_forms
            && enough_choices(
                card.arabic,
                others()
                    .filter(|other| other.has_forms)
                    .map(|other| other.arabic),
            );
        let asks_listening =
            card.has_audio && enough_choices(card.arabic, others().map(|other| other.arabic));
        asks_meaning || asks_form || asks_listening
    })
}

fn enough_choices<'a>(answer: &str, others: impl Iterator<Item = &'a str>) -> bool {
    others
        .filter(|other| *other != answer)
        .collect::<HashSet<_>>()
        .len()
        >= 2
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::{ExerciseCard, has_exercise, passes};

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        name: String,
        cards: Vec<CaseCard>,
        has_exercise: bool,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CaseCard {
        arabic: String,
        meaning_fr: Option<String>,
        #[serde(default)]
        forms: bool,
        #[serde(default)]
        audio: bool,
    }

    #[test]
    fn agrees_with_the_webapp_on_every_shared_case() {
        let cases: Vec<Case> = serde_json::from_str(include_str!(
            "../../convertisseur-webapp/src/__tests__/exerciseCases.json"
        ))
        .unwrap();

        for case in cases {
            let cards: Vec<ExerciseCard<'_>> = case
                .cards
                .iter()
                .map(|card| ExerciseCard {
                    arabic: &card.arabic,
                    meaning: card.meaning_fr.as_deref(),
                    has_forms: card.forms,
                    has_audio: card.audio,
                })
                .collect();
            assert_eq!(has_exercise(&cards), case.has_exercise, "{}", case.name);
        }
    }

    #[test]
    fn passes_from_ninety_percent() {
        assert!(passes(9, 10));
        assert!(passes(11, 12));
        assert!(!passes(10, 12));
        assert!(passes(12, 12));
        assert!(!passes(0, 0));
    }
}
