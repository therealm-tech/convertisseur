-- What a reviewer reports about an item, e.g. a wrong vowel; NULL when
-- there is nothing to report.
ALTER TABLE units ADD COLUMN review_note text
CHECK (review_note <> '' AND char_length(review_note) <= 2000);
ALTER TABLE lessons ADD COLUMN review_note text
CHECK (review_note <> '' AND char_length(review_note) <= 2000);
ALTER TABLE cards ADD COLUMN review_note text
CHECK (review_note <> '' AND char_length(review_note) <= 2000);

-- The review list also shows where an item comes from and its note.
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
    units.review_note
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
    lessons.title AS search_text,
    lessons.import_key,
    lessons.review_note
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
    concat_ws(' ', cards.arabic, cards.transliteration, cards.meaning_fr) AS search_text,
    cards.import_key,
    cards.review_note
FROM cards
INNER JOIN lessons ON cards.lesson_id = lessons.id
INNER JOIN units ON lessons.unit_id = units.id;
