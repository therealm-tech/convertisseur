//! The content as learners see it: what is validated, together with every
//! row above it. A lesson validated in a unit still to review is not shown,
//! nor are its cards.

use uuid::Uuid;

use crate::{
    content::{Track, Variety},
    letters::FormColumns,
};

/// A row of the outline: a visible unit, and one of its visible lessons when
/// it has any.
#[derive(sqlx::FromRow)]
pub struct OutlineRow {
    pub track: Track,
    pub unit_id: Uuid,
    pub unit_title: String,
    pub lesson_id: Option<Uuid>,
    pub lesson_title: Option<String>,
    pub card_count: i64,
}

/// A track, with its visible units in order.
pub struct Outline {
    pub track: Track,
    pub units: Vec<OutlineUnit>,
}

pub struct OutlineUnit {
    pub id: Uuid,
    pub title: String,
    pub lessons: Vec<OutlineLesson>,
}

pub struct OutlineLesson {
    pub id: Uuid,
    pub title: String,
    /// How many of its cards learners see.
    pub card_count: i64,
}

/// Groups rows ordered by track, unit then lesson into one outline per track
/// of `tracks`, in that order; a track with no row gets no units.
#[must_use]
pub fn outlines(tracks: &[Track], rows: Vec<OutlineRow>) -> Vec<Outline> {
    let mut outlines: Vec<Outline> = tracks
        .iter()
        .map(|&track| Outline {
            track,
            units: Vec::new(),
        })
        .collect();
    for row in rows {
        let Some(outline) = outlines.iter_mut().find(|o| o.track == row.track) else {
            continue;
        };
        if outline
            .units
            .last()
            .is_none_or(|unit| unit.id != row.unit_id)
        {
            outline.units.push(OutlineUnit {
                id: row.unit_id,
                title: row.unit_title,
                lessons: Vec::new(),
            });
        }
        if let (Some(id), Some(title), Some(unit)) =
            (row.lesson_id, row.lesson_title, outline.units.last_mut())
        {
            unit.lessons.push(OutlineLesson {
                id,
                title,
                card_count: row.card_count,
            });
        }
    }
    outlines
}

/// A lesson learners see.
#[derive(sqlx::FromRow)]
pub struct StudyLesson {
    pub id: Uuid,
    pub track: Track,
    pub unit_id: Uuid,
    pub unit_title: String,
    pub title: String,
    pub audio_id: Option<Uuid>,
    pub audio_duration_ms: Option<i32>,
}

/// A card learners see.
#[derive(sqlx::FromRow)]
pub struct StudyCard {
    pub id: Uuid,
    pub arabic: String,
    pub transliteration: Option<String>,
    pub meaning_fr: Option<String>,
    pub variety: Variety,
    #[sqlx(flatten)]
    pub forms: FormColumns,
    pub audio_id: Option<Uuid>,
    pub audio_duration_ms: Option<i32>,
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{OutlineRow, outlines};
    use crate::content::Track;

    fn row(track: Track, unit: u128, lesson: Option<u128>) -> OutlineRow {
        OutlineRow {
            track,
            unit_id: Uuid::from_u128(unit),
            unit_title: format!("unit {unit}"),
            lesson_id: lesson.map(Uuid::from_u128),
            lesson_title: lesson.map(|id| format!("lesson {id}")),
            card_count: 0,
        }
    }

    #[test]
    fn groups_lessons_under_their_unit_in_order() {
        let rows = vec![
            row(Track::Arabe, 1, Some(10)),
            row(Track::Arabe, 1, Some(11)),
            row(Track::Arabe, 2, None),
            row(Track::Islam, 3, Some(12)),
        ];

        let outlines = outlines(&[Track::Arabe, Track::Islam], rows);

        let arabe = &outlines[0];
        assert_eq!(arabe.track, Track::Arabe);
        let units: Vec<_> = arabe.units.iter().map(|u| u.id.as_u128()).collect();
        assert_eq!(units, [1, 2]);
        let lessons: Vec<_> = arabe.units[0]
            .lessons
            .iter()
            .map(|l| l.id.as_u128())
            .collect();
        assert_eq!(lessons, [10, 11]);
        assert!(arabe.units[1].lessons.is_empty());
        assert_eq!(outlines[1].units[0].lessons[0].id.as_u128(), 12);
    }

    #[test]
    fn keeps_a_track_without_units() {
        let outlines = outlines(&[Track::Arabe, Track::Islam], vec![]);

        assert_eq!(outlines.len(), 2);
        assert!(outlines.iter().all(|o| o.units.is_empty()));
    }

    #[test]
    fn leaves_out_rows_of_other_tracks() {
        let outlines = outlines(&[Track::Islam], vec![row(Track::Arabe, 1, None)]);

        assert_eq!(outlines.len(), 1);
        assert!(outlines[0].units.is_empty());
    }
}
