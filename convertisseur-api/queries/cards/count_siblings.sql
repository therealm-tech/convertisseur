SELECT count(*)
FROM cards
WHERE lesson_id = $1 AND id IS DISTINCT FROM $2;
