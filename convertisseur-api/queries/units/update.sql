WITH edit AS (
    SELECT id
    FROM units
    WHERE
        id = $1
        AND (track, title)
        IS DISTINCT FROM ($2::content_track, $4::text)
)

UPDATE units
SET
    track = $2,
    position = $3,
    title = $4,
    status = CASE WHEN EXISTS (SELECT 1 FROM edit) THEN 'to_review' ELSE units.status END,
    validated_by = CASE WHEN EXISTS (SELECT 1 FROM edit) THEN NULL ELSE units.validated_by END,
    validated_at = CASE WHEN EXISTS (SELECT 1 FROM edit) THEN NULL ELSE units.validated_at END,
    updated_by = $5,
    updated_at = now()
WHERE
    id = $1
    AND (units.track, units.position, units.title)
    IS DISTINCT FROM ($2::content_track, $3::integer, $4::text);
