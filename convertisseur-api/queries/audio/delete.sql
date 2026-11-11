DELETE FROM audio
WHERE id = $1
RETURNING object_key;
