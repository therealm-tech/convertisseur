SELECT
    track AS parent,
    position
FROM units
WHERE id = $1
FOR UPDATE;
