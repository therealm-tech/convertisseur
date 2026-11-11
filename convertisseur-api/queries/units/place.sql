-- Sets positions in bulk; a row flagged in $3 was moved, which records who
-- moved it and when, the others only shift.
UPDATE units
SET
    position = placed.position,
    updated_by = CASE WHEN placed.moved THEN $4 ELSE units.updated_by END,
    updated_at = CASE WHEN placed.moved THEN now() ELSE units.updated_at END
FROM
    unnest($1::uuid[], $2::integer[], $3::boolean[])
        AS placed (id, position, moved)
WHERE units.id = placed.id;
