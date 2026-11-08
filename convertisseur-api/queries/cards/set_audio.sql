-- A new recording is an edit: the cards goes back to review.
UPDATE cards
SET
    audio_id = $2,
    status = 'to_review',
    validated_by = NULL,
    validated_at = NULL,
    updated_by = $3,
    updated_at = now()
WHERE id = $1;
