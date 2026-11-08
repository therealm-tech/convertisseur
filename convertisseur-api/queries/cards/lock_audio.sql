SELECT audio_id
FROM cards
WHERE id = $1
FOR UPDATE;
