UPDATE units
SET review_note = $2
WHERE id = $1
RETURNING review_note;
