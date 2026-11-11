UPDATE units
SET position = position + $3
WHERE track = $1 AND position >= $2 AND id IS DISTINCT FROM $4;
