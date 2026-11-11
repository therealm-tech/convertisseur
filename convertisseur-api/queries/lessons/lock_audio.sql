SELECT audio_id
FROM lessons
WHERE id = $1
FOR UPDATE;
