DELETE FROM lessons
WHERE id = $1
RETURNING unit_id AS parent, position, audio_id;
