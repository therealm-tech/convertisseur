UPDATE lessons
SET position = position + $3
WHERE unit_id = $1 AND position >= $2 AND id IS DISTINCT FROM $4;
