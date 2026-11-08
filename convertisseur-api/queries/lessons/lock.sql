SELECT
    unit_id AS parent,
    position
FROM lessons
WHERE id = $1
FOR UPDATE;
