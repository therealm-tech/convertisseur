SELECT
    lessons.id,
    lessons.unit_id,
    units.position AS unit_position,
    units.title AS unit_title,
    units.track,
    lessons.position,
    lessons.title,
    lessons.status,
    lessons.created_at,
    lessons.created_by,
    lessons.updated_at,
    lessons.updated_by,
    lessons.validated_at,
    lessons.validated_by
FROM lessons
INNER JOIN units ON lessons.unit_id = units.id
WHERE lessons.id = $1;
