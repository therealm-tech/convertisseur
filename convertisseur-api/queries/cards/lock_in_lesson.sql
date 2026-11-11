-- The cards among $2 that belong to the lesson $1, locked until the end of
-- the transaction.
SELECT id
FROM cards
WHERE lesson_id = $1 AND id = any($2)
FOR UPDATE;
