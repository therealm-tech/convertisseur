-- Validates the cards among $2 of the lesson $1 that are to review without
-- a review note, by the account $3; flagged and validated cards stay as
-- they are.
UPDATE cards
SET
    status = 'validated',
    validated_by = $3,
    validated_at = now()
WHERE
    lesson_id = $1
    AND id = any($2)
    AND status = 'to_review'
    AND review_note IS NULL;
