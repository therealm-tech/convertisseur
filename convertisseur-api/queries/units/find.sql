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
    validated_by,
    import_key,
    review_note
FROM units
WHERE id = $1;
