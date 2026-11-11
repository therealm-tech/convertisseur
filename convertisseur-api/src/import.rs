//! `import-content`: creates the units, lessons and cards of the content
//! files that the database lacks, to review. What exists, by its import key,
//! is left as it is, so that edits made in the admin area always win.

mod catalogue;

use anyhow::{Context, anyhow};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

pub use catalogue::{Catalogue, InvalidCatalogue};

use crate::{
    content::{self, CardFields, ContentRefusal, LessonFields, UnitFields},
    db::{
        self,
        content::{Cards, Lessons, Ordered, Units},
    },
};

/// What an import did, for one kind of row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub created: usize,
    /// Already in the database, left as it is.
    pub skipped: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub units: Tally,
    pub lessons: Tally,
    pub cards: Tally,
}

/// Imports the content files built into the binary.
///
/// # Errors
///
/// Fails when a content file is invalid, when there is no super admin to
/// author the content, or when the database cannot be written; nothing is
/// imported then.
pub async fn import_content(pool: &PgPool) -> anyhow::Result<ImportReport> {
    let catalogue = Catalogue::embedded()?;
    let (units, lessons, cards) = catalogue.counts();
    tracing::debug!(units, lessons, cards, "content files read");
    import(pool, &catalogue).await
}

/// Creates, in one transaction, every entry of `catalogue` whose key no row
/// carries, last among its siblings and authored by the super admin.
///
/// # Errors
///
/// Fails when there is no super admin or the database cannot be written;
/// nothing is imported then.
pub async fn import(pool: &PgPool, catalogue: &Catalogue) -> anyhow::Result<ImportReport> {
    let mut tx = pool.begin().await?;
    db::content::lock(&mut *tx).await?;
    let author = db::users::find_super_admin(&mut *tx)
        .await?
        .context("no super admin account: start the server once, which creates it")?;
    let mut report = ImportReport::default();
    for unit in &catalogue.units {
        let fields = UnitFields {
            track: unit.track,
            title: unit.title.clone(),
        };
        let unit_id = ensure::<Units>(
            &mut tx,
            unit.track,
            &unit.key,
            &mut report.units,
            async |conn, position| {
                db::content::insert_unit(conn, &fields, position, Some(&unit.key), author).await
            },
        )
        .await?;
        for lesson in &unit.lessons {
            let fields = LessonFields {
                unit_id,
                title: lesson.title.clone(),
            };
            let lesson_id = ensure::<Lessons>(
                &mut tx,
                unit_id,
                &lesson.key,
                &mut report.lessons,
                async |conn, position| {
                    db::content::insert_lesson(conn, &fields, position, Some(&lesson.key), author)
                        .await
                },
            )
            .await?;
            for card in &lesson.cards {
                let fields = CardFields {
                    lesson_id,
                    arabic: card.arabic.clone(),
                    transliteration: card.transliteration.clone(),
                    meaning_fr: card.meaning_fr.clone(),
                    variety: card.variety,
                };
                ensure::<Cards>(
                    &mut tx,
                    lesson_id,
                    &card.key,
                    &mut report.cards,
                    async |conn, position| {
                        db::content::insert_card(conn, &fields, position, Some(&card.key), author)
                            .await
                    },
                )
                .await?;
            }
        }
    }
    tx.commit().await?;
    tracing::info!(
        author_id = %author,
        created_units = report.units.created,
        created_lessons = report.lessons.created,
        created_cards = report.cards.created,
        skipped_units = report.units.skipped,
        skipped_lessons = report.lessons.skipped,
        skipped_cards = report.cards.skipped,
        "content import finished"
    );
    Ok(report)
}

/// The id of the row imported under `key`, created last under `parent` when
/// no row carries it.
async fn ensure<O: Ordered>(
    conn: &mut PgConnection,
    parent: O::Parent,
    key: &str,
    tally: &mut Tally,
    insert: impl AsyncFnOnce(&mut PgConnection, i32) -> sqlx::Result<Uuid>,
) -> anyhow::Result<Uuid> {
    if let Some(id) = db::content::find_by_import_key::<O>(&mut *conn, key).await? {
        tracing::info!(kind = ?O::KIND, key, %id, "content already present, skipped");
        tally.skipped += 1;
        return Ok(id);
    }
    let position = content::place::<O>(conn, parent, None, None)
        .await
        .map_err(|refusal| match refusal {
            ContentRefusal::Failed(error) => error,
            other => anyhow!("cannot place {key}: {other:?}"),
        })?;
    let id = insert(conn, position).await?;
    tracing::info!(kind = ?O::KIND, key, %id, position, "content created, to review");
    tally.created += 1;
    Ok(id)
}
