//! The content files under `content/`: one YAML file per unit, in the
//! directory of its track, read in path order.

use std::{collections::HashSet, fmt};

use serde::Deserialize;

use crate::content::{Track, Variety};

/// The content files, embedded at build time by `build.rs`.
static FILES: &[(&str, &str)] = include!(concat!(env!("OUT_DIR"), "/content_files.rs"));

/// Every unit of the content files, in order. Keys are paths of the
/// entries' own keys: `<track>/<unit>`, `<track>/<unit>/<lesson>`,
/// `<track>/<unit>/<lesson>/<card>`.
pub struct Catalogue {
    pub units: Vec<UnitEntry>,
}

pub struct UnitEntry {
    pub key: String,
    pub track: Track,
    pub title: String,
    pub lessons: Vec<LessonEntry>,
}

pub struct LessonEntry {
    pub key: String,
    pub title: String,
    pub cards: Vec<CardEntry>,
}

pub struct CardEntry {
    pub key: String,
    pub arabic: String,
    pub transliteration: Option<String>,
    pub meaning_fr: Option<String>,
    pub variety: Variety,
}

/// Every problem found in the content files, each prefixed with its file.
#[derive(Debug)]
pub struct InvalidCatalogue(pub Vec<String>);

impl fmt::Display for InvalidCatalogue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid content files:")?;
        for problem in &self.0 {
            write!(f, "\n  {problem}")?;
        }
        Ok(())
    }
}

impl std::error::Error for InvalidCatalogue {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitFile {
    key: String,
    title: String,
    variety: Variety,
    lessons: Vec<LessonFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LessonFile {
    key: String,
    title: String,
    cards: Vec<CardFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CardFile {
    key: String,
    arabic: String,
    transliteration: Option<String>,
    meaning_fr: Option<String>,
}

impl Catalogue {
    /// The content files built into the binary.
    ///
    /// # Errors
    ///
    /// Fails with every problem found when a file does not follow the model.
    pub fn embedded() -> Result<Self, InvalidCatalogue> {
        Self::parse(FILES)
    }

    /// Parses and checks `files`, given as `(path under content/, text)`.
    ///
    /// # Errors
    ///
    /// Fails with every problem found when a file does not follow the model.
    pub fn parse(files: &[(&str, &str)]) -> Result<Self, InvalidCatalogue> {
        let mut problems = Vec::new();
        let mut units = Vec::new();
        let mut unit_keys = HashSet::new();
        for (path, text) in files {
            let mut report = |problem: String| problems.push(format!("{path}: {problem}"));
            let Some(track) = track_of(path) else {
                report("a content file sits directly in content/arabe/ or content/islam/".into());
                continue;
            };
            let file: UnitFile = match serde_saphyr::from_str(text) {
                Ok(file) => file,
                Err(error) => {
                    report(error.to_string());
                    continue;
                }
            };
            units.push(check_unit(track, file, &mut unit_keys, &mut report));
        }
        if problems.is_empty() {
            Ok(Self { units })
        } else {
            Err(InvalidCatalogue(problems))
        }
    }

    /// How many units, lessons and cards the files hold.
    #[must_use]
    pub fn counts(&self) -> (usize, usize, usize) {
        let lessons = self.units.iter().flat_map(|u| &u.lessons);
        let cards = lessons.clone().map(|l| l.cards.len()).sum();
        (self.units.len(), lessons.count(), cards)
    }
}

fn track_of(path: &str) -> Option<Track> {
    match path.split_once('/')? {
        ("arabe", name) if !name.contains('/') => Some(Track::Arabe),
        ("islam", name) if !name.contains('/') => Some(Track::Islam),
        _ => None,
    }
}

fn check_unit(
    track: Track,
    file: UnitFile,
    unit_keys: &mut HashSet<String>,
    report: &mut impl FnMut(String),
) -> UnitEntry {
    let track_name = match track {
        Track::Arabe => "arabe",
        Track::Islam => "islam",
    };
    let key = child_key(track_name, &file.key, unit_keys, report);
    check_text("title", &file.title, &key, report);
    if track == Track::Islam && file.variety != Variety::Classique {
        report(format!("{key}: the islam track is in classical Arabic"));
    }
    if file.lessons.is_empty() {
        report(format!("{key}: a unit has at least one lesson"));
    }
    let mut lesson_keys = HashSet::new();
    let lessons = file
        .lessons
        .into_iter()
        .map(|lesson| {
            let lesson_key = child_key(&key, &lesson.key, &mut lesson_keys, report);
            check_text("title", &lesson.title, &lesson_key, report);
            if lesson.cards.is_empty() {
                report(format!("{lesson_key}: a lesson has at least one card"));
            }
            let mut card_keys = HashSet::new();
            let cards = lesson
                .cards
                .into_iter()
                .map(|card| {
                    let card_key = child_key(&lesson_key, &card.key, &mut card_keys, report);
                    check_arabic(&card.arabic, &card_key, report);
                    for (field, value) in [
                        ("transliteration", &card.transliteration),
                        ("meaning_fr", &card.meaning_fr),
                    ] {
                        if let Some(value) = value {
                            check_text(field, value, &card_key, report);
                        }
                    }
                    CardEntry {
                        key: card_key,
                        arabic: card.arabic,
                        transliteration: card.transliteration,
                        meaning_fr: card.meaning_fr,
                        variety: file.variety,
                    }
                })
                .collect();
            LessonEntry {
                key: lesson_key,
                title: lesson.title,
                cards,
            }
        })
        .collect();
    UnitEntry {
        key,
        track,
        title: file.title,
        lessons,
    }
}

/// The full key of an entry under `parent`, reporting a malformed key or one
/// its siblings already use.
fn child_key(
    parent: &str,
    key: &str,
    siblings: &mut HashSet<String>,
    report: &mut impl FnMut(String),
) -> String {
    let full = format!("{parent}/{key}");
    if !is_key(key) {
        report(format!(
            "{full}: a key is lowercase ASCII letters and digits, in words joined by `-`"
        ));
    }
    if !siblings.insert(full.clone()) {
        report(format!("{full}: the key is used twice"));
    }
    full
}

fn is_key(key: &str) -> bool {
    !key.is_empty()
        && key.split('-').all(|word| {
            !word.is_empty()
                && word
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

fn check_text(field: &str, value: &str, key: &str, report: &mut impl FnMut(String)) {
    if value.trim().is_empty() {
        report(format!("{key}: {field} is blank"));
    } else if value.trim() != value {
        report(format!("{key}: {field} starts or ends with blank space"));
    }
}

/// Arabic text holds Arabic letters, and no Latin ones.
fn check_arabic(value: &str, key: &str, report: &mut impl FnMut(String)) {
    check_text("arabic", value, key, report);
    if !value.chars().any(is_arabic_letter) {
        report(format!("{key}: arabic holds no Arabic letter"));
    }
    if value.chars().any(|c| c.is_ascii_alphabetic()) {
        report(format!("{key}: arabic holds Latin letters"));
    }
}

fn is_arabic_letter(c: char) -> bool {
    matches!(c, '\u{0621}'..='\u{064A}' | '\u{0671}'..='\u{06D3}')
}

#[cfg(test)]
mod tests {
    use super::{Catalogue, FILES};
    use crate::content::{Track, Variety};

    const CARD: &str = "
key: unit
title: Unit
variety: classique
lessons:
  - key: lesson
    title: Lesson
    cards:
      - key: card
        arabic: بَيْتٌ
        transliteration: bayt
        meaning_fr: Une maison
";

    const NONE: [String; 0] = [];

    fn problems(files: &[(&str, &str)]) -> Vec<String> {
        match Catalogue::parse(files) {
            Ok(_) => Vec::new(),
            Err(invalid) => invalid.0,
        }
    }

    fn assert_refused(files: &[(&str, &str)], expected: &str) {
        let problems = problems(files);
        assert!(
            problems.iter().any(|p| p.contains(expected)),
            "expected a problem containing {expected:?}, got {problems:?}"
        );
    }

    #[test]
    fn the_content_files_are_valid() {
        let catalogue = Catalogue::embedded().unwrap_or_else(|invalid| panic!("{invalid}"));
        assert_eq!(FILES.len(), catalogue.units.len());
        let (units, lessons, cards) = catalogue.counts();
        assert!(units > 0 && lessons > 0 && cards > 0);
        for unit in &catalogue.units {
            let varieties = unit
                .lessons
                .iter()
                .flat_map(|l| &l.cards)
                .map(|c| c.variety);
            if unit.track == Track::Islam {
                assert!(varieties.into_iter().all(|v| v == Variety::Classique));
            }
        }
    }

    #[test]
    fn builds_keys_from_the_track_down() {
        let catalogue = Catalogue::parse(&[("islam/01-unit.yaml", CARD)]).unwrap();
        let unit = &catalogue.units[0];
        assert_eq!(unit.key, "islam/unit");
        assert_eq!(unit.track, Track::Islam);
        assert_eq!(unit.lessons[0].key, "islam/unit/lesson");
        let card = &unit.lessons[0].cards[0];
        assert_eq!(card.key, "islam/unit/lesson/card");
        assert_eq!(card.variety, Variety::Classique);
        assert_eq!(card.meaning_fr.as_deref(), Some("Une maison"));
    }

    #[test]
    fn refuses_a_key_used_twice_among_siblings() {
        let twice = CARD.replace(
            "        meaning_fr: Une maison\n",
            "        meaning_fr: Une maison\n      - key: card\n        arabic: مَاءٌ\n",
        );
        assert_refused(
            &[("arabe/01-unit.yaml", &twice)],
            "arabe/unit/lesson/card: the key is used twice",
        );
        assert_refused(
            &[("arabe/01-unit.yaml", CARD), ("arabe/02-other.yaml", CARD)],
            "arabe/unit: the key is used twice",
        );
    }

    #[test]
    fn accepts_the_same_key_under_different_parents() {
        assert_eq!(
            problems(&[("arabe/01-unit.yaml", CARD), ("islam/01-unit.yaml", CARD)]),
            NONE
        );
    }

    #[test]
    fn refuses_a_malformed_key() {
        assert_refused(
            &[(
                "arabe/01-unit.yaml",
                &CARD.replace("key: card", "key: Card 1"),
            )],
            "a key is lowercase",
        );
    }

    #[test]
    fn refuses_blank_or_latin_arabic() {
        assert_refused(
            &[(
                "arabe/01-unit.yaml",
                &CARD.replace("arabic: بَيْتٌ", "arabic: '  '"),
            )],
            "arabic is blank",
        );
        assert_refused(
            &[(
                "arabe/01-unit.yaml",
                &CARD.replace("arabic: بَيْتٌ", "arabic: bayt"),
            )],
            "arabic holds no Arabic letter",
        );
    }

    #[test]
    fn refuses_darija_in_the_islam_track() {
        let darija = CARD.replace("variety: classique", "variety: darija");
        assert_eq!(problems(&[("arabe/01-unit.yaml", &darija)]), NONE);
        assert_refused(
            &[("islam/01-unit.yaml", &darija)],
            "the islam track is in classical Arabic",
        );
    }

    #[test]
    fn refuses_an_unknown_variety_or_field() {
        assert_refused(
            &[(
                "arabe/01-unit.yaml",
                &CARD.replace("variety: classique", "variety: kabyle"),
            )],
            "arabe/01-unit.yaml: ",
        );
        assert_refused(
            &[(
                "arabe/01-unit.yaml",
                &CARD.replace("transliteration:", "explication:"),
            )],
            "explication",
        );
    }

    #[test]
    fn refuses_a_file_outside_a_track() {
        assert_refused(
            &[("01-unit.yaml", CARD)],
            "01-unit.yaml: a content file sits",
        );
        assert_refused(&[("divers/01-unit.yaml", CARD)], "divers/01-unit.yaml");
    }

    #[test]
    fn refuses_an_empty_lesson() {
        let empty = CARD.split("    cards:").next().unwrap().to_owned() + "    cards: []\n";
        assert_refused(
            &[("arabe/01-unit.yaml", &empty)],
            "a lesson has at least one card",
        );
    }
}
