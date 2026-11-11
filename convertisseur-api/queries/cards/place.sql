-- Sets parents and positions in bulk; a row flagged in $4 was moved, which
-- records who moved it and when, the others only shift.
UPDATE cards
SET
    lesson_id = placed.parent,
    position = placed.position,
    updated_by = CASE WHEN placed.moved THEN $5 ELSE cards.updated_by END,
    updated_at = CASE WHEN placed.moved THEN now() ELSE cards.updated_at END
FROM
    unnest($1::uuid[], $2::uuid[], $3::integer[], $4::boolean[])
        AS placed (id, parent, position, moved)
WHERE cards.id = placed.id;
