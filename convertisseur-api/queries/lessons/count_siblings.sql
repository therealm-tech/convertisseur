SELECT count(*)
FROM lessons
WHERE unit_id = $1 AND id IS DISTINCT FROM $2;
