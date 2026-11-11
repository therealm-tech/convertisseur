-- The content files' text replaces the unit's: it goes back to review, and
-- its review note, answered by the change, is cleared.
UPDATE units
SET
    track = $2,
    title = $3,
    status = 'to_review',
    validated_by = NULL,
    validated_at = NULL,
    review_note = NULL,
    updated_by = $4,
    updated_at = now()
WHERE id = $1;
