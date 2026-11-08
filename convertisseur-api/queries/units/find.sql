SELECT
    id,
    track,
    position,
    title,
    status,
    created_at,
    created_by,
    updated_at,
    updated_by,
    validated_at,
    validated_by
FROM units
WHERE id = $1;
