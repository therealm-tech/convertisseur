-- Sets parents and positions in bulk; a row flagged in $4 was moved, which
-- records who moved it and when, the others only shift.
UPDATE lessons
SET
    unit_id = placed.parent,
    position = placed.position,
    updated_by = CASE WHEN placed.moved THEN $5 ELSE lessons.updated_by END,
    updated_at = CASE WHEN placed.moved THEN now() ELSE lessons.updated_at END
FROM
    unnest($1::uuid[], $2::uuid[], $3::integer[], $4::boolean[])
        AS placed (id, parent, position, moved)
WHERE lessons.id = placed.id;
