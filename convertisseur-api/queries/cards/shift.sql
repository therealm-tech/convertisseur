UPDATE cards
SET position = position + $3
WHERE lesson_id = $1 AND position >= $2 AND id IS DISTINCT FROM $4;
