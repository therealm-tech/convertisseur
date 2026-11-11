-- The content files' text replaces the lesson's: it goes back to review, and
-- its review note, answered by the change, is cleared.
UPDATE lessons
SET
    title = $2,
    status = 'to_review',
    validated_by = NULL,
    validated_at = NULL,
    review_note = NULL,
    updated_by = $3,
    updated_at = now()
WHERE id = $1;
