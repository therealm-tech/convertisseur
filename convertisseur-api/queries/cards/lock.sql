SELECT
    lesson_id AS parent,
    position
FROM cards
WHERE id = $1
FOR UPDATE;
