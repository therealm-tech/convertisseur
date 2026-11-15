-- A recording is reviewed on its own: the card's status stays as it is.
UPDATE cards
SET
    audio_id = $2,
    updated_by = $3,
    updated_at = now()
WHERE id = $1;
