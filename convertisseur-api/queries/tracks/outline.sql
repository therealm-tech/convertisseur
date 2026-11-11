-- The units learners see, each with the lessons they see in it, in order:
-- one row per lesson, or one row with no lesson for a unit that has none.
-- A row is visible when it and every row above it are validated.
SELECT
    units.track,
    units.id AS unit_id,
    units.title AS unit_title,
    lessons.id AS lesson_id,
    lessons.title AS lesson_title,
    (
        SELECT count(*)
        FROM cards
        WHERE cards.lesson_id = lessons.id AND cards.status = 'validated'
    ) AS card_count
FROM units
LEFT JOIN lessons ON units.id = lessons.unit_id AND lessons.status = 'validated'
WHERE
    units.status = 'validated'
    AND ($1::content_track IS NULL OR units.track = $1)
ORDER BY units.track ASC, units.position ASC, lessons.position ASC;
