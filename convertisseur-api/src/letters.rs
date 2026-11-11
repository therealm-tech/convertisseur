//! The letters of the alphabet and the shapes they take within a word.

use std::fmt;

/// The kashida, `ـ`: in a written form, it stands for the letters the shape
/// joins.
pub const TATWEEL: char = '\u{0640}';

/// How a letter joins its neighbours, which decides the shapes it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Joining {
    /// Joins the letters on both sides: initial, medial and final shapes.
    Both,
    /// Joins the letter before it, never the next one: a final shape alone.
    PreviousOnly,
    /// Only ever ends a word: a final shape alone.
    WordFinal,
}

impl Joining {
    /// How `letter` joins, `None` when it is not a single letter with
    /// forms: a word, a vowelled letter, the hamza on its own.
    #[must_use]
    pub fn of(letter: &str) -> Option<Self> {
        let mut chars = letter.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            return None;
        };
        match c {
            'ا' | 'أ' | 'إ' | 'آ' | 'ٱ' | 'د' | 'ذ' | 'ر' | 'ز' | 'و' | 'ؤ' => {
                Some(Self::PreviousOnly)
            }
            'ة' | 'ى' => Some(Self::WordFinal),
            'ب' | 'ت' | 'ث' | 'ج' | 'ح' | 'خ' | 'س' | 'ش' | 'ص' | 'ض' | 'ط' | 'ظ' | 'ع' | 'غ'
            | 'ف' | 'ق' | 'ك' | 'ل' | 'م' | 'ن' | 'ه' | 'ي' | 'ئ' | 'ڤ' | 'ڨ' | 'ڭ' | 'گ' => {
                Some(Self::Both)
            }
            _ => None,
        }
    }
}

/// The shapes of a letter besides the isolated one, which is the card's
/// Arabic. A letter with an initial shape always has a medial one, so the
/// two come together or not at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LetterForms {
    Joining {
        initial: String,
        medial: String,
        r#final: String,
    },
    FinalOnly {
        r#final: String,
    },
}

/// The forms as written, before they are checked against their letter.
pub struct WrittenForms<'a> {
    pub initial: Option<&'a str>,
    pub medial: Option<&'a str>,
    pub r#final: &'a str,
}

#[derive(Clone, Copy)]
enum Position {
    Initial,
    Medial,
    Final,
}

impl Position {
    /// The letter in this position: the tatweel stands on the side it joins.
    fn shape(self, letter: &str) -> String {
        match self {
            Self::Initial => format!("{letter}{TATWEEL}"),
            Self::Medial => format!("{TATWEEL}{letter}{TATWEEL}"),
            Self::Final => format!("{TATWEEL}{letter}"),
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Initial => "initial",
            Self::Medial => "medial",
            Self::Final => "final",
        })
    }
}

impl Position {
    fn tatweel(self) -> &'static str {
        match self {
            Self::Initial => "the tatweel after the letter",
            Self::Medial => "the tatweel on both sides",
            Self::Final => "the tatweel before the letter",
        }
    }
}

impl LetterForms {
    /// Checks `forms` against `letter`: each form is the letter with the
    /// tatweel where it joins, and a letter that never joins the next one
    /// has no initial or medial form.
    ///
    /// # Errors
    ///
    /// Every problem found, in words.
    pub fn check(letter: &str, forms: &WrittenForms<'_>) -> Result<Self, Vec<String>> {
        let Some(joining) = Joining::of(letter) else {
            return Err(vec![format!(
                "forms belong to a single letter of the alphabet, and «{letter}» is not one"
            )]);
        };
        let mut problems = Vec::new();
        let mut expect = |position: Position, written: &str| {
            let shape = position.shape(letter);
            if written != shape {
                problems.push(format!(
                    "the {position} form of {letter} is «{shape}», {}, not «{written}»",
                    position.tatweel()
                ));
            }
        };
        expect(Position::Final, forms.r#final);
        let checked = match (joining, forms.initial, forms.medial) {
            (Joining::Both, Some(initial), Some(medial)) => {
                expect(Position::Initial, initial);
                expect(Position::Medial, medial);
                Some(Self::Joining {
                    initial: initial.to_owned(),
                    medial: medial.to_owned(),
                    r#final: forms.r#final.to_owned(),
                })
            }
            (Joining::Both, _, _) => {
                problems.push(format!(
                    "{letter} joins the next letter: it has initial, medial and final forms"
                ));
                None
            }
            (Joining::PreviousOnly | Joining::WordFinal, None, None) => Some(Self::FinalOnly {
                r#final: forms.r#final.to_owned(),
            }),
            (Joining::PreviousOnly, _, _) => {
                problems.push(format!(
                    "{letter} never joins the next letter: it has a final form alone"
                ));
                None
            }
            (Joining::WordFinal, _, _) => {
                problems.push(format!(
                    "{letter} only ever ends a word: it has a final form alone"
                ));
                None
            }
        };
        match checked {
            Some(forms) if problems.is_empty() => Ok(forms),
            _ => Err(problems),
        }
    }

    /// The forms as three columns, initial, medial and final.
    #[must_use]
    pub fn columns(forms: Option<&Self>) -> [Option<&str>; 3] {
        match forms {
            None => [None; 3],
            Some(Self::Joining {
                initial,
                medial,
                r#final,
            }) => [Some(initial), Some(medial), Some(r#final)],
            Some(Self::FinalOnly { r#final }) => [None, None, Some(r#final)],
        }
    }
}

/// The forms as the `cards` table stores them.
#[derive(sqlx::FromRow)]
pub struct FormColumns {
    #[sqlx(rename = "initial_form")]
    pub initial: Option<String>,
    #[sqlx(rename = "medial_form")]
    pub medial: Option<String>,
    #[sqlx(rename = "final_form")]
    pub r#final: Option<String>,
}

impl FormColumns {
    /// The forms the columns hold; the table's check constraint rules out
    /// the combinations that would read as none.
    #[must_use]
    pub fn into_forms(self) -> Option<LetterForms> {
        match (self.initial, self.medial, self.r#final) {
            (Some(initial), Some(medial), Some(r#final)) => Some(LetterForms::Joining {
                initial,
                medial,
                r#final,
            }),
            (None, None, Some(r#final)) => Some(LetterForms::FinalOnly { r#final }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FormColumns, Joining, LetterForms, WrittenForms};

    fn check(
        letter: &str,
        initial: Option<&str>,
        medial: Option<&str>,
        r#final: &str,
    ) -> Result<LetterForms, Vec<String>> {
        LetterForms::check(
            letter,
            &WrittenForms {
                initial,
                medial,
                r#final,
            },
        )
    }

    fn refused(result: Result<LetterForms, Vec<String>>, expected: &str) {
        let problems = result.expect_err("the forms should be refused");
        assert!(
            problems.iter().any(|p| p.contains(expected)),
            "expected a problem containing {expected:?}, got {problems:?}"
        );
    }

    #[test]
    fn sorts_the_letters_by_how_they_join() {
        assert_eq!(Joining::of("ب"), Some(Joining::Both));
        assert_eq!(Joining::of("ه"), Some(Joining::Both));
        for letter in ["ا", "د", "ذ", "ر", "ز", "و"] {
            assert_eq!(Joining::of(letter), Some(Joining::PreviousOnly), "{letter}");
        }
        assert_eq!(Joining::of("ة"), Some(Joining::WordFinal));
        assert_eq!(Joining::of("ى"), Some(Joining::WordFinal));
        for not_a_letter in ["ء", "بَ", "بت", "", "b"] {
            assert_eq!(Joining::of(not_a_letter), None, "{not_a_letter}");
        }
    }

    #[test]
    fn accepts_the_four_shapes_of_a_joining_letter() {
        assert_eq!(
            check("ب", Some("بـ"), Some("ـبـ"), "ـب"),
            Ok(LetterForms::Joining {
                initial: "بـ".into(),
                medial: "ـبـ".into(),
                r#final: "ـب".into(),
            })
        );
    }

    #[test]
    fn accepts_a_final_form_alone_for_a_letter_that_does_not_join_the_next() {
        assert_eq!(
            check("ر", None, None, "ـر"),
            Ok(LetterForms::FinalOnly {
                r#final: "ـر".into()
            })
        );
        assert_eq!(
            check("ة", None, None, "ـة"),
            Ok(LetterForms::FinalOnly {
                r#final: "ـة".into()
            })
        );
    }

    #[test]
    fn refuses_a_misplaced_or_missing_tatweel() {
        refused(
            check("ب", Some("ـب"), Some("ـبـ"), "ـب"),
            "the initial form of ب is «بـ», the tatweel after the letter, not «ـب»",
        );
        refused(
            check("ب", Some("بـ"), Some("بـ"), "ـب"),
            "the medial form of ب is «ـبـ», the tatweel on both sides",
        );
        refused(
            check("ب", Some("بـ"), Some("ـبـ"), "ب"),
            "the final form of ب is «ـب», the tatweel before the letter",
        );
    }

    #[test]
    fn refuses_a_form_of_another_letter() {
        refused(
            check("ب", Some("تـ"), Some("ـبـ"), "ـب"),
            "is «بـ», the tatweel after the letter, not «تـ»",
        );
    }

    #[test]
    fn refuses_an_initial_or_medial_form_for_a_letter_that_does_not_join_the_next() {
        refused(
            check("د", Some("دـ"), None, "ـد"),
            "د never joins the next letter",
        );
        refused(
            check("ا", Some("اـ"), Some("ـاـ"), "ـا"),
            "ا never joins the next letter",
        );
        refused(
            check("ى", None, Some("ـىـ"), "ـى"),
            "ى only ever ends a word",
        );
    }

    #[test]
    fn refuses_a_medial_form_without_an_initial_one() {
        refused(
            check("ب", None, Some("ـبـ"), "ـب"),
            "ب joins the next letter: it has initial, medial and final forms",
        );
        refused(
            check("ب", None, None, "ـب"),
            "it has initial, medial and final forms",
        );
    }

    #[test]
    fn refuses_forms_for_what_is_not_a_single_letter() {
        refused(check("بَيْت", None, None, "ـبيت"), "«بَيْت» is not one");
        refused(check("ء", None, None, "ـء"), "«ء» is not one");
    }

    #[test]
    fn round_trips_through_the_columns() {
        for forms in [
            LetterForms::Joining {
                initial: "بـ".into(),
                medial: "ـبـ".into(),
                r#final: "ـب".into(),
            },
            LetterForms::FinalOnly {
                r#final: "ـا".into(),
            },
        ] {
            let [initial, medial, last] = LetterForms::columns(Some(&forms));
            let columns = FormColumns {
                initial: initial.map(Into::into),
                medial: medial.map(Into::into),
                r#final: last.map(Into::into),
            };
            assert_eq!(columns.into_forms(), Some(forms));
        }
        assert_eq!(LetterForms::columns(None), [None; 3]);
        let none = FormColumns {
            initial: None,
            medial: None,
            r#final: None,
        };
        assert_eq!(none.into_forms(), None);
    }
}
