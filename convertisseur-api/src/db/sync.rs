use sqlx::{PgExecutor, Postgres};
use uuid::Uuid;

use crate::{
    content::{Track, Variety},
    letters::{FormColumns, LetterForms},
    sync::{CardText, LessonText, Row, UnitText},
};

#[derive(sqlx::FromRow)]
struct UnitRow {
    id: Uuid,
    parent: Track,
    position: i32,
    import_key: Option<String>,
    title: String,
    review_note: Option<String>,
}

#[derive(sqlx::FromRow)]
struct LessonRow {
    id: Uuid,
    parent: Uuid,
    position: i32,
    import_key: Option<String>,
    title: String,
    review_note: Option<String>,
}

#[derive(sqlx::FromRow)]
struct CardRow {
    id: Uuid,
    parent: Uuid,
    position: i32,
    import_key: Option<String>,
    arabic: String,
    transliteration: Option<String>,
    meaning_fr: Option<String>,
    variety: Variety,
    #[sqlx(flatten)]
    forms: FormColumns,
    review_note: Option<String>,
}

/// Every unit, imported or not.
pub async fn units(exec: impl PgExecutor<'_>) -> sqlx::Result<Vec<Row<Track, UnitText>>> {
    let rows: Vec<UnitRow> = sqlx::query_as(include_str!("../../queries/units/snapshot.sql"))
        .fetch_all(exec)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| Row {
            id: row.id,
            parent: row.parent,
            position: row.position,
            import_key: row.import_key,
            text: UnitText {
                track: row.parent,
                title: row.title,
            },
            review_note: row.review_note,
        })
        .collect())
}

/// Every lesson, imported or not.
pub async fn lessons(exec: impl PgExecutor<'_>) -> sqlx::Result<Vec<Row<Uuid, LessonText>>> {
    let rows: Vec<LessonRow> = sqlx::query_as(include_str!("../../queries/lessons/snapshot.sql"))
        .fetch_all(exec)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| Row {
            id: row.id,
            parent: row.parent,
            position: row.position,
            import_key: row.import_key,
            text: LessonText { title: row.title },
            review_note: row.review_note,
        })
        .collect())
}

/// Every card, imported or not.
pub async fn cards(exec: impl PgExecutor<'_>) -> sqlx::Result<Vec<Row<Uuid, CardText>>> {
    let rows: Vec<CardRow> = sqlx::query_as(include_str!("../../queries/cards/snapshot.sql"))
        .fetch_all(exec)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| Row {
            id: row.id,
            parent: row.parent,
            position: row.position,
            import_key: row.import_key,
            text: CardText {
                arabic: row.arabic,
                transliteration: row.transliteration,
                meaning_fr: row.meaning_fr,
                variety: row.variety,
                forms: row.forms.into_forms(),
            },
            review_note: row.review_note,
        })
        .collect())
}

/// Rewrites the unit's text, back to review, its note cleared.
pub async fn overwrite_unit(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    text: &UnitText,
    editor: Uuid,
) -> sqlx::Result<()> {
    sqlx::query(include_str!("../../queries/units/overwrite.sql"))
        .bind(id)
        .bind(text.track)
        .bind(&text.title)
        .bind(editor)
        .execute(exec)
        .await?;
    Ok(())
}

/// Rewrites the lesson's text, back to review, its note cleared.
pub async fn overwrite_lesson(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    text: &LessonText,
    editor: Uuid,
) -> sqlx::Result<()> {
    sqlx::query(include_str!("../../queries/lessons/overwrite.sql"))
        .bind(id)
        .bind(&text.title)
        .bind(editor)
        .execute(exec)
        .await?;
    Ok(())
}

/// Rewrites the card's text, back to review, its note cleared.
pub async fn overwrite_card(
    exec: impl PgExecutor<'_>,
    id: Uuid,
    text: &CardText,
    editor: Uuid,
) -> sqlx::Result<()> {
    let [initial, medial, last] = LetterForms::columns(text.forms.as_ref());
    sqlx::query(include_str!("../../queries/cards/overwrite.sql"))
        .bind(id)
        .bind(&text.arabic)
        .bind(&text.transliteration)
        .bind(&text.meaning_fr)
        .bind(text.variety)
        .bind(editor)
        .bind(initial)
        .bind(medial)
        .bind(last)
        .execute(exec)
        .await?;
    Ok(())
}

/// Where a row goes: under `parent`, at `position`; `moved` records the
/// move as an edit by `editor`, where a row that only shifts keeps its
/// last edit.
pub struct Place<P> {
    pub id: Uuid,
    pub parent: P,
    pub position: i32,
    pub moved: bool,
}

/// Places units; a unit's track is its text, which [`overwrite_unit`]
/// writes, so only its position changes here.
pub async fn place_units(
    exec: impl PgExecutor<'_>,
    places: &[Place<Track>],
    editor: Uuid,
) -> sqlx::Result<()> {
    sqlx::query(include_str!("../../queries/units/place.sql"))
        .bind(places.iter().map(|p| p.id).collect::<Vec<_>>())
        .bind(places.iter().map(|p| p.position).collect::<Vec<_>>())
        .bind(places.iter().map(|p| p.moved).collect::<Vec<_>>())
        .bind(editor)
        .execute(exec)
        .await?;
    Ok(())
}

/// Places lessons or cards, according to `query`, under their parents.
pub async fn place_children(
    exec: impl PgExecutor<'_>,
    query: &'static str,
    places: &[Place<Uuid>],
    editor: Uuid,
) -> sqlx::Result<()> {
    sqlx::query::<Postgres>(query)
        .bind(places.iter().map(|p| p.id).collect::<Vec<_>>())
        .bind(places.iter().map(|p| p.parent).collect::<Vec<_>>())
        .bind(places.iter().map(|p| p.position).collect::<Vec<_>>())
        .bind(places.iter().map(|p| p.moved).collect::<Vec<_>>())
        .bind(editor)
        .execute(exec)
        .await?;
    Ok(())
}

pub const PLACE_LESSONS: &str = include_str!("../../queries/lessons/place.sql");
pub const PLACE_CARDS: &str = include_str!("../../queries/cards/place.sql");
