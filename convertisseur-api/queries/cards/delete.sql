DELETE FROM cards
WHERE id = $1
RETURNING lesson_id AS parent, position;
