-- One page of the lesson's cards, in order.
SELECT id
FROM cards
WHERE lesson_id = $1
ORDER BY position ASC
LIMIT $2 OFFSET $3;
