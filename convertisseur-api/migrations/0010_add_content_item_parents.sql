-- The review list links an item to its unit and lesson, and its text search
-- also matches the parent's title.
CREATE OR REPLACE VIEW content_items AS
SELECT
    'unit'::content_kind AS kind,
    units.id,
    units.track,
    units.title,
    NULL::text AS transliteration,
    NULL::text AS parent_title,
    units.status,
    units.updated_at,
    units.title AS search_text,
    units.import_key,
    units.review_note,
    NULL::uuid AS unit_id,
    NULL::uuid AS lesson_id
FROM units
UNION ALL
SELECT
    'lesson'::content_kind AS kind,
    lessons.id,
    units.track,
    lessons.title,
    NULL::text AS transliteration,
    units.title AS parent_title,
    lessons.status,
    lessons.updated_at,
    concat_ws(' ', lessons.title, units.title) AS search_text,
    lessons.import_key,
    lessons.review_note,
    units.id AS unit_id,
    NULL::uuid AS lesson_id
FROM lessons
INNER JOIN units ON lessons.unit_id = units.id
UNION ALL
SELECT
    'card'::content_kind AS kind,
    cards.id,
    units.track,
    cards.arabic AS title,
    cards.transliteration,
    lessons.title AS parent_title,
    cards.status,
    cards.updated_at,
    concat_ws(
        ' ', cards.arabic, cards.transliteration, cards.meaning_fr, lessons.title
    ) AS search_text,
    cards.import_key,
    cards.review_note,
    units.id AS unit_id,
    lessons.id AS lesson_id
FROM cards
INNER JOIN lessons ON cards.lesson_id = lessons.id
INNER JOIN units ON lessons.unit_id = units.id;
