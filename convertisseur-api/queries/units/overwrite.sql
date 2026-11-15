-- The content files' text replaces the unit's: the files are reviewed in
-- pull requests, so it is validated by the author of the sync, and its
-- review note, answered by the change, is cleared.
UPDATE units
SET
    track = $2,
    title = $3,
    status = 'validated',
    validated_by = $4,
    validated_at = now(),
    review_note = NULL,
    updated_by = $4,
    updated_at = now()
WHERE id = $1;
