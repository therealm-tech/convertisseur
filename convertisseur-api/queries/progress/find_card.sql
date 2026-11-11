-- A card learners see, with the lesson and track it belongs to.
SELECT
    cards.lesson_id,
    units.track
FROM cards
INNER JOIN lessons ON cards.lesson_id = lessons.id
INNER JOIN units ON lessons.unit_id = units.id
WHERE
    cards.id = $1
    AND cards.status = 'validated'
    AND lessons.status = 'validated'
    AND units.status = 'validated';
