UPDATE lessons
SET
    unit_id = $2,
    position = $3,
    title = $4,
    status = 'to_review',
    validated_by = NULL,
    validated_at = NULL,
    updated_by = $5,
    updated_at = now()
WHERE
    id = $1
    AND (unit_id, position, title) IS DISTINCT FROM ($2::uuid, $3::integer, $4::text);
