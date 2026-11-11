-- The review state of the cards among $1, in their lesson's order.
SELECT
    id,
    status,
    validated_at,
    validated_by,
    review_note
FROM cards
WHERE id = any($1)
ORDER BY position ASC;
