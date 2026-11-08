UPDATE units
SET
    track = $2,
    position = $3,
    title = $4,
    status = 'to_review',
    validated_by = NULL,
    validated_at = NULL,
    updated_by = $5,
    updated_at = now()
WHERE
    id = $1
    AND (track, position, title) IS DISTINCT FROM ($2::content_track, $3::integer, $4::text);
