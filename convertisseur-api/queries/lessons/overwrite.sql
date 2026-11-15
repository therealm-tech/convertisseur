-- The content files' text replaces the lesson's: the files are reviewed in
-- pull requests, so it is validated by the author of the sync, and its
-- review note, answered by the change, is cleared.
UPDATE lessons
SET
    title = $2,
    status = 'validated',
    validated_by = $3,
    validated_at = now(),
    review_note = NULL,
    updated_by = $3,
    updated_at = now()
WHERE id = $1;
