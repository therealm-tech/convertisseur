//! Brings the units, lessons and cards of the content files into the
//! database at every start-up: the files are the only source of what they
//! hold, so an imported row follows them in text, place and existence.

mod catalogue;

use std::{
    collections::{HashMap, HashSet},
    fmt,
    hash::Hash,
};

use anyhow::Context;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

pub use catalogue::{Catalogue, InvalidCatalogue};

use crate::{
    content::{self, CardFields, LessonFields, Track, UnitFields, Variety},
    db::{
        self,
        content::{Cards, Lessons, Ordered, Units},
        sync::Place,
    },
    storage::Storage,
};

/// What a sync did, for one kind of row. A row both rewritten and moved
/// counts as updated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub created: usize,
    /// Its text changed in the files: rewritten, back to review.
    pub updated: usize,
    /// Its parent, or its order among its imported siblings, changed.
    pub moved: usize,
    /// Gone from the files.
    pub deleted: usize,
    pub unchanged: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SyncReport {
    pub units: Tally,
    pub lessons: Tally,
    pub cards: Tally,
}

/// Entries gone from the content files still hold content written in the
/// admin area; each line names an entry and what it holds.
#[derive(Debug)]
pub struct HeldByRemovedEntries(pub Vec<String>);

impl fmt::Display for HeldByRemovedEntries {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "entries gone from the content files still hold content written in the admin \
             area; move or delete that content in the admin area, or put the entries back in \
             content/, then start the server again:"
        )?;
        for line in &self.0 {
            write!(f, "\n  {line}")?;
        }
        Ok(())
    }
}

impl std::error::Error for HeldByRemovedEntries {}

/// A row as the sync finds it, imported or written in the admin area.
pub struct Row<P, T> {
    pub id: Uuid,
    pub parent: P,
    pub position: i32,
    pub import_key: Option<String>,
    pub text: T,
    pub review_note: Option<String>,
}

/// What the files say of a unit. Its track is both its text and its
/// parent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitText {
    pub track: Track,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LessonText {
    pub title: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardText {
    pub arabic: String,
    pub transliteration: Option<String>,
    pub meaning_fr: Option<String>,
    pub variety: Variety,
}

/// Syncs the content files built into the binary.
///
/// # Errors
///
/// Fails when a content file is invalid, when there is no super admin to
/// author the content, when entries gone from the files hold content
/// written in the admin area ([`HeldByRemovedEntries`]), or when the
/// database cannot be written; nothing changes then.
pub async fn sync_content(pool: &PgPool, storage: &Storage) -> anyhow::Result<SyncReport> {
    let catalogue = Catalogue::embedded()?;
    let (units, lessons, cards) = catalogue.counts();
    tracing::debug!(units, lessons, cards, "content files read");
    sync(pool, storage, &catalogue).await
}

/// Makes the imported rows match `catalogue`, in one transaction under the
/// content lock: creates what is missing, rewrites what changed, puts every
/// row in its place, and deletes what is gone, with its recordings. The
/// objects of the deleted recordings go after the commit.
///
/// # Errors
///
/// As [`sync_content`], the content files aside.
pub async fn sync(
    pool: &PgPool,
    storage: &Storage,
    catalogue: &Catalogue,
) -> anyhow::Result<SyncReport> {
    let mut tx = pool.begin().await?;
    db::content::lock(&mut *tx).await?;
    let author = db::users::find_super_admin(&mut *tx)
        .await?
        .context("no super admin account: the server creates it before the content")?;
    let units_before = db::sync::units(&mut *tx).await?;
    let lessons_before = db::sync::lessons(&mut *tx).await?;
    let cards_before = db::sync::cards(&mut *tx).await?;
    let mut report = SyncReport::default();

    let wanted = catalogue.units.iter().map(|unit| Wanted {
        key: unit.key.clone(),
        parent: unit.track,
        text: UnitText {
            track: unit.track,
            title: unit.title.clone(),
        },
    });
    let units = write::<Units>(&mut tx, author, &units_before, wanted, &mut report.units).await?;
    let wanted = catalogue.units.iter().flat_map(|unit| {
        let unit_id = units.ids[&unit.key];
        unit.lessons.iter().map(move |lesson| Wanted {
            key: lesson.key.clone(),
            parent: unit_id,
            text: LessonText {
                title: lesson.title.clone(),
            },
        })
    });
    let lessons = write::<Lessons>(
        &mut tx,
        author,
        &lessons_before,
        wanted,
        &mut report.lessons,
    )
    .await?;
    let wanted = catalogue
        .units
        .iter()
        .flat_map(|unit| &unit.lessons)
        .flat_map(|lesson| {
            let lesson_id = lessons.ids[&lesson.key];
            lesson.cards.iter().map(move |card| Wanted {
                key: card.key.clone(),
                parent: lesson_id,
                text: CardText {
                    arabic: card.arabic.clone(),
                    transliteration: card.transliteration.clone(),
                    meaning_fr: card.meaning_fr.clone(),
                    variety: card.variety,
                },
            })
        });
    let cards = write::<Cards>(&mut tx, author, &cards_before, wanted, &mut report.cards).await?;

    let mut held = held_by(&units.removed, "unit", &lessons_before, |t| &t.title);
    held.extend(held_by(&lessons.removed, "lesson", &cards_before, |t| {
        &t.arabic
    }));
    if !held.is_empty() {
        return Err(HeldByRemovedEntries(held).into());
    }

    arrange_level::<Units>(&mut tx, author, &units_before, &units, &mut report.units).await?;
    arrange_level::<Lessons>(
        &mut tx,
        author,
        &lessons_before,
        &lessons,
        &mut report.lessons,
    )
    .await?;
    arrange_level::<Cards>(&mut tx, author, &cards_before, &cards, &mut report.cards).await?;

    let mut objects = Vec::new();
    remove::<Cards>(&mut tx, &cards, &mut report.cards, &mut objects).await?;
    remove::<Lessons>(&mut tx, &lessons, &mut report.lessons, &mut objects).await?;
    remove::<Units>(&mut tx, &units, &mut report.units, &mut objects).await?;
    tx.commit().await?;
    for key in &objects {
        content::delete_object(storage, key).await;
    }

    log_summary(author, &report, objects.len());
    Ok(report)
}

/// Logs what a sync did, in one line.
fn log_summary(author: Uuid, report: &SyncReport, recordings_deleted: usize) {
    let SyncReport {
        units,
        lessons,
        cards,
    } = *report;
    tracing::info!(
        author_id = %author,
        units_created = units.created,
        units_updated = units.updated,
        units_moved = units.moved,
        units_deleted = units.deleted,
        units_unchanged = units.unchanged,
        lessons_created = lessons.created,
        lessons_updated = lessons.updated,
        lessons_moved = lessons.moved,
        lessons_deleted = lessons.deleted,
        lessons_unchanged = lessons.unchanged,
        cards_created = cards.created,
        cards_updated = cards.updated,
        cards_moved = cards.moved,
        cards_deleted = cards.deleted,
        cards_unchanged = cards.unchanged,
        recordings_deleted,
        "content synced"
    );
}

/// A content table the sync writes.
trait Level: Ordered<Parent: Eq + Hash + fmt::Debug> {
    type Text: PartialEq;

    async fn insert(
        conn: &mut PgConnection,
        parent: Self::Parent,
        text: &Self::Text,
        position: i32,
        key: &str,
        author: Uuid,
    ) -> sqlx::Result<Uuid>;

    async fn overwrite(
        conn: &mut PgConnection,
        id: Uuid,
        text: &Self::Text,
        author: Uuid,
    ) -> sqlx::Result<()>;

    async fn place(
        conn: &mut PgConnection,
        places: &[Place<Self::Parent>],
        author: Uuid,
    ) -> sqlx::Result<()>;
}

impl Level for Units {
    type Text = UnitText;

    async fn insert(
        conn: &mut PgConnection,
        parent: Track,
        text: &UnitText,
        position: i32,
        key: &str,
        author: Uuid,
    ) -> sqlx::Result<Uuid> {
        let fields = UnitFields {
            track: parent,
            title: text.title.clone(),
        };
        db::content::insert_unit(conn, &fields, position, Some(key), author).await
    }

    async fn overwrite(
        conn: &mut PgConnection,
        id: Uuid,
        text: &UnitText,
        author: Uuid,
    ) -> sqlx::Result<()> {
        db::sync::overwrite_unit(conn, id, text, author).await
    }

    async fn place(
        conn: &mut PgConnection,
        places: &[Place<Track>],
        author: Uuid,
    ) -> sqlx::Result<()> {
        db::sync::place_units(conn, places, author).await
    }
}

impl Level for Lessons {
    type Text = LessonText;

    async fn insert(
        conn: &mut PgConnection,
        parent: Uuid,
        text: &LessonText,
        position: i32,
        key: &str,
        author: Uuid,
    ) -> sqlx::Result<Uuid> {
        let fields = LessonFields {
            unit_id: parent,
            title: text.title.clone(),
        };
        db::content::insert_lesson(conn, &fields, position, Some(key), author).await
    }

    async fn overwrite(
        conn: &mut PgConnection,
        id: Uuid,
        text: &LessonText,
        author: Uuid,
    ) -> sqlx::Result<()> {
        db::sync::overwrite_lesson(conn, id, text, author).await
    }

    async fn place(
        conn: &mut PgConnection,
        places: &[Place<Uuid>],
        author: Uuid,
    ) -> sqlx::Result<()> {
        db::sync::place_children(conn, db::sync::PLACE_LESSONS, places, author).await
    }
}

impl Level for Cards {
    type Text = CardText;

    async fn insert(
        conn: &mut PgConnection,
        parent: Uuid,
        text: &CardText,
        position: i32,
        key: &str,
        author: Uuid,
    ) -> sqlx::Result<Uuid> {
        let fields = CardFields {
            lesson_id: parent,
            arabic: text.arabic.clone(),
            transliteration: text.transliteration.clone(),
            meaning_fr: text.meaning_fr.clone(),
            variety: text.variety,
        };
        db::content::insert_card(conn, &fields, position, Some(key), author).await
    }

    async fn overwrite(
        conn: &mut PgConnection,
        id: Uuid,
        text: &CardText,
        author: Uuid,
    ) -> sqlx::Result<()> {
        db::sync::overwrite_card(conn, id, text, author).await
    }

    async fn place(
        conn: &mut PgConnection,
        places: &[Place<Uuid>],
        author: Uuid,
    ) -> sqlx::Result<()> {
        db::sync::place_children(conn, db::sync::PLACE_CARDS, places, author).await
    }
}

/// An entry of the files, under the row of its parent's key.
struct Wanted<P, T> {
    key: String,
    parent: P,
    text: T,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Change {
    Created,
    Updated,
    Unchanged,
}

/// An imported row once its text is written, in the files' order.
struct Imported<P> {
    id: Uuid,
    key: String,
    parent: P,
    change: Change,
}

/// What [`write`] did to one table.
struct Written<P> {
    imported: Vec<Imported<P>>,
    /// The id of every entry of the files, by key.
    ids: HashMap<String, Uuid>,
    /// The imported rows gone from the files: id and key.
    removed: Vec<(Uuid, String)>,
}

/// Creates the entries no row carries the key of, and rewrites the rows
/// whose text differs from their entry's. Places come later, in
/// [`arrange_level`]: a new row takes a provisional position, which the
/// deferred uniqueness of positions tolerates until then.
async fn write<L: Level>(
    conn: &mut PgConnection,
    author: Uuid,
    before: &[Row<L::Parent, L::Text>],
    wanted: impl Iterator<Item = Wanted<L::Parent, L::Text>>,
    tally: &mut Tally,
) -> anyhow::Result<Written<L::Parent>> {
    let existing: HashMap<&str, &Row<L::Parent, L::Text>> = before
        .iter()
        .filter_map(|row| Some((row.import_key.as_deref()?, row)))
        .collect();
    let mut imported = Vec::new();
    for (index, entry) in wanted.enumerate() {
        let key = entry.key.as_str();
        let (id, change) = match existing.get(key) {
            Some(row) if row.text == entry.text => (row.id, Change::Unchanged),
            Some(row) => {
                L::overwrite(conn, row.id, &entry.text, author).await?;
                tracing::info!(
                    kind = ?L::KIND, key, id = %row.id,
                    "content rewritten from the files, to review"
                );
                if row.review_note.is_some() {
                    tracing::info!(
                        kind = ?L::KIND, key, id = %row.id,
                        "review note cleared by the rewrite"
                    );
                }
                tally.updated += 1;
                (row.id, Change::Updated)
            }
            None => {
                let position = i32::try_from(index + 1).unwrap_or(i32::MAX);
                let id = L::insert(conn, entry.parent, &entry.text, position, key, author).await?;
                tracing::info!(kind = ?L::KIND, key, %id, "content created, to review");
                tally.created += 1;
                (id, Change::Created)
            }
        };
        imported.push(Imported {
            id,
            key: entry.key,
            parent: entry.parent,
            change,
        });
    }
    let ids: HashMap<String, Uuid> = imported.iter().map(|i| (i.key.clone(), i.id)).collect();
    let removed = before
        .iter()
        .filter_map(|row| {
            let key = row.import_key.as_ref()?;
            (!ids.contains_key(key)).then(|| (row.id, key.clone()))
        })
        .collect();
    Ok(Written {
        imported,
        ids,
        removed,
    })
}

/// The rows written in the admin area that hang off a removed row, one line
/// each: deleting the removed row would take them with it.
fn held_by<T>(
    removed: &[(Uuid, String)],
    kind: &str,
    children: &[Row<Uuid, T>],
    label: impl Fn(&T) -> &str,
) -> Vec<String> {
    let removed: HashMap<Uuid, &str> = removed
        .iter()
        .map(|(id, key)| (*id, key.as_str()))
        .collect();
    children
        .iter()
        .filter(|child| child.import_key.is_none())
        .filter_map(|child| {
            let key = removed.get(&child.parent)?;
            Some(format!(
                "{key} ({kind}) holds « {} » ({}), written in the admin area",
                label(&child.text),
                child.id
            ))
        })
        .collect()
}

/// Puts every row of the table in its place, see [`arrange`], and counts
/// the imported rows that moved or stayed as they were.
async fn arrange_level<L: Level>(
    conn: &mut PgConnection,
    author: Uuid,
    before: &[Row<L::Parent, L::Text>],
    written: &Written<L::Parent>,
    tally: &mut Tally,
) -> anyhow::Result<()> {
    let current: Vec<Sibling<L::Parent>> = before
        .iter()
        .map(|row| Sibling {
            id: row.id,
            parent: row.parent,
            position: row.position,
            imported: row.import_key.is_some(),
        })
        .collect();
    let wanted: Vec<(Uuid, L::Parent)> =
        written.imported.iter().map(|i| (i.id, i.parent)).collect();
    let removed: HashSet<Uuid> = written.removed.iter().map(|(id, _)| *id).collect();
    let was: HashMap<Uuid, (L::Parent, i32)> = before
        .iter()
        .map(|row| (row.id, (row.parent, row.position)))
        .collect();
    let imported: HashMap<Uuid, &Imported<L::Parent>> =
        written.imported.iter().map(|i| (i.id, i)).collect();
    let mut places = Vec::new();
    for arranged in arrange(&current, &wanted, &removed) {
        if let Some(row) = imported.get(&arranged.id) {
            if arranged.moved {
                tracing::info!(
                    kind = ?L::KIND, key = row.key, id = %row.id, parent = ?arranged.parent,
                    position = arranged.position, "content moved as in the files"
                );
            }
            match row.change {
                Change::Created | Change::Updated => {}
                Change::Unchanged if arranged.moved => tally.moved += 1,
                Change::Unchanged => tally.unchanged += 1,
            }
        }
        if arranged.moved || was.get(&arranged.id) != Some(&(arranged.parent, arranged.position)) {
            places.push(Place {
                id: arranged.id,
                parent: arranged.parent,
                position: arranged.position,
                moved: arranged.moved,
            });
        }
    }
    tracing::debug!(kind = ?L::KIND, rows = places.len(), "content placed");
    L::place(conn, &places, author).await?;
    Ok(())
}

/// Deletes the imported rows gone from the files, and their recordings,
/// whose objects are added to `objects` to delete after the commit.
async fn remove<L: Level>(
    conn: &mut PgConnection,
    written: &Written<L::Parent>,
    tally: &mut Tally,
    objects: &mut Vec<String>,
) -> anyhow::Result<()> {
    for (id, key) in &written.removed {
        let deleted = db::content::delete::<L>(&mut *conn, *id).await?;
        if let Some(audio_id) = deleted.and_then(|d| d.audio_id)
            && let Some(object) = db::audio::delete(&mut *conn, audio_id).await?
        {
            objects.push(object);
        }
        tracing::info!(kind = ?L::KIND, key, %id, "content deleted: gone from the files");
        tally.deleted += 1;
    }
    Ok(())
}

/// A row where it stands before the sync.
#[derive(Clone, Copy)]
struct Sibling<P> {
    id: Uuid,
    parent: P,
    position: i32,
    imported: bool,
}

/// A row where the sync puts it. `moved` is set on an imported row that
/// changes parent, or that leaves its order among the imported siblings it
/// keeps; a row that only shifts for a sibling created, deleted or moved is
/// not moved.
#[derive(Debug, PartialEq, Eq)]
struct Arranged<P> {
    id: Uuid,
    parent: P,
    position: i32,
    moved: bool,
}

/// Orders the siblings of every parent. The imported rows, `wanted` with
/// their parent in the files' order, take that order; a row written in the
/// admin area stays right after the imported sibling it followed, or first
/// when none came before it, and rows following the same one keep their
/// order. Rows in `removed` take no place.
fn arrange<P: Copy + Eq + Hash>(
    current: &[Sibling<P>],
    wanted: &[(Uuid, P)],
    removed: &HashSet<Uuid>,
) -> Vec<Arranged<P>> {
    let mut parents: Vec<P> = Vec::new();
    let mut before: HashMap<P, Vec<Sibling<P>>> = HashMap::new();
    let mut after: HashMap<P, Vec<Uuid>> = HashMap::new();
    for sibling in current {
        if !before.contains_key(&sibling.parent) {
            parents.push(sibling.parent);
        }
        before.entry(sibling.parent).or_default().push(*sibling);
    }
    for &(id, parent) in wanted {
        if !before.contains_key(&parent) && !after.contains_key(&parent) {
            parents.push(parent);
        }
        after.entry(parent).or_default().push(id);
    }
    let wanted_parent: HashMap<Uuid, P> = wanted.iter().copied().collect();
    let existed: HashSet<Uuid> = current.iter().map(|s| s.id).collect();

    let mut arranged = Vec::new();
    for parent in parents {
        let mut previous = before.remove(&parent).unwrap_or_default();
        previous.sort_by_key(|s| (s.position, s.id));
        let imported = after.remove(&parent).unwrap_or_default();

        let mut followers: HashMap<Option<Uuid>, Vec<Uuid>> = HashMap::new();
        let mut anchor = None;
        for sibling in &previous {
            if !sibling.imported {
                followers.entry(anchor).or_default().push(sibling.id);
            } else if wanted_parent.get(&sibling.id) == Some(&parent) {
                anchor = Some(sibling.id);
            }
        }

        let rank: HashMap<Uuid, usize> = previous
            .iter()
            .enumerate()
            .map(|(rank, s)| (s.id, rank))
            .collect();
        let ranks: Vec<usize> = imported
            .iter()
            .filter_map(|id| rank.get(id).copied())
            .collect();
        let kept = longest_increasing(&ranks);

        let mut order = followers.remove(&None).unwrap_or_default();
        for &id in &imported {
            order.push(id);
            order.extend(followers.remove(&Some(id)).unwrap_or_default());
        }
        for (index, id) in order.into_iter().enumerate() {
            let moved = wanted_parent.contains_key(&id)
                && match rank.get(&id) {
                    Some(rank) => !kept.contains(rank),
                    None => existed.contains(&id),
                };
            arranged.push(Arranged {
                id,
                parent,
                position: i32::try_from(index + 1).unwrap_or(i32::MAX),
                moved,
            });
        }
    }
    arranged.retain(|a| !removed.contains(&a.id));
    arranged
}

/// The values of one longest strictly increasing subsequence of `values`,
/// which are distinct: the imported rows that keep their order, the others
/// being the fewest that moved.
fn longest_increasing(values: &[usize]) -> HashSet<usize> {
    let mut length = vec![1_usize; values.len()];
    let mut previous = vec![None; values.len()];
    for i in 0..values.len() {
        for j in 0..i {
            if values[j] < values[i] && length[j] + 1 > length[i] {
                length[i] = length[j] + 1;
                previous[i] = Some(j);
            }
        }
    }
    let mut kept = HashSet::new();
    let mut cursor = (0..values.len()).rev().max_by_key(|&i| length[i]);
    while let Some(i) = cursor {
        kept.insert(values[i]);
        cursor = previous[i];
    }
    kept
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use uuid::Uuid;

    use super::{Arranged, Sibling, arrange};

    const LESSON: u8 = 100;
    const OTHER: u8 = 200;

    fn id(n: u8) -> Uuid {
        Uuid::from_bytes([n; 16])
    }

    /// Siblings under `parent`, in order: `true` for an imported one.
    fn siblings(parent: u8, rows: &[(u8, bool)]) -> Vec<Sibling<Uuid>> {
        rows.iter()
            .zip(1..)
            .map(|(&(n, imported), position)| Sibling {
                id: id(n),
                parent: id(parent),
                position,
                imported,
            })
            .collect()
    }

    fn wanted(parent: u8, rows: &[u8]) -> Vec<(Uuid, Uuid)> {
        rows.iter().map(|&n| (id(n), id(parent))).collect()
    }

    /// The rows under `parent`, in order, and those that moved.
    fn under(arranged: &[Arranged<Uuid>], parent: u8) -> (Vec<u8>, Vec<u8>) {
        let mut rows: Vec<&Arranged<Uuid>> =
            arranged.iter().filter(|a| a.parent == id(parent)).collect();
        rows.sort_by_key(|a| a.position);
        let positions: Vec<i32> = rows.iter().map(|a| a.position).collect();
        let dense: Vec<i32> = (1..=i32::try_from(rows.len()).unwrap()).collect();
        assert_eq!(positions, dense);
        let name = |a: &&Arranged<Uuid>| a.id.as_bytes()[0];
        (
            rows.iter().map(name).collect(),
            rows.iter().filter(|a| a.moved).map(name).collect(),
        )
    }

    #[test]
    fn imported_rows_take_the_files_order() {
        let current = siblings(LESSON, &[(1, true), (2, true), (3, true)]);
        let arranged = arrange(&current, &wanted(LESSON, &[3, 1, 2]), &HashSet::new());
        assert_eq!(under(&arranged, LESSON), (vec![3, 1, 2], vec![3]));
    }

    #[test]
    fn a_handwritten_row_stays_after_the_imported_row_it_followed() {
        let current = siblings(LESSON, &[(1, true), (10, false), (11, false), (2, true)]);
        let arranged = arrange(&current, &wanted(LESSON, &[2, 1]), &HashSet::new());
        assert_eq!(under(&arranged, LESSON).0, vec![2, 1, 10, 11]);
    }

    #[test]
    fn a_handwritten_row_before_every_imported_row_stays_first() {
        let current = siblings(LESSON, &[(10, false), (1, true), (2, true)]);
        let arranged = arrange(&current, &wanted(LESSON, &[3, 1, 2]), &HashSet::new());
        assert_eq!(under(&arranged, LESSON), (vec![10, 3, 1, 2], vec![]));
    }

    #[test]
    fn a_handwritten_row_follows_the_previous_imported_row_when_its_own_goes() {
        let current = siblings(LESSON, &[(1, true), (2, true), (10, false), (3, true)]);
        let removed = HashSet::from([id(2)]);
        let arranged = arrange(&current, &wanted(LESSON, &[1, 3]), &removed);
        assert_eq!(under(&arranged, LESSON), (vec![1, 10, 3], vec![]));
    }

    #[test]
    fn creations_and_deletions_move_nothing() {
        let current = siblings(LESSON, &[(1, true), (2, true), (3, true)]);
        let removed = HashSet::from([id(2)]);
        let arranged = arrange(&current, &wanted(LESSON, &[4, 1, 3]), &removed);
        assert_eq!(under(&arranged, LESSON), (vec![4, 1, 3], vec![]));
    }

    #[test]
    fn a_swap_moves_one_row() {
        let current = siblings(LESSON, &[(1, true), (2, true), (3, true), (4, true)]);
        let arranged = arrange(&current, &wanted(LESSON, &[1, 3, 2, 4]), &HashSet::new());
        let (order, moved) = under(&arranged, LESSON);
        assert_eq!(order, vec![1, 3, 2, 4]);
        assert_eq!(moved.len(), 1);
    }

    #[test]
    fn a_row_changing_parent_moves_and_closes_its_gap() {
        let mut current = siblings(LESSON, &[(1, true), (2, true), (3, true)]);
        current.extend(siblings(OTHER, &[(4, true), (10, false)]));
        let mut files = wanted(LESSON, &[1, 3]);
        files.extend(wanted(OTHER, &[4, 2]));
        let arranged = arrange(&current, &files, &HashSet::new());
        assert_eq!(under(&arranged, LESSON), (vec![1, 3], vec![]));
        assert_eq!(under(&arranged, OTHER), (vec![4, 10, 2], vec![2]));
        let parents: HashMap<Uuid, Uuid> = arranged.iter().map(|a| (a.id, a.parent)).collect();
        assert_eq!(parents[&id(2)], id(OTHER));
    }
}
