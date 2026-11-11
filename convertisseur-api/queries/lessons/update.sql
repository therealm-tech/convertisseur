WITH edit AS (
    SELECT id
    FROM lessons
    WHERE
        id = $1
        AND title IS DISTINCT FROM $4::text
)

UPDATE lessons
SET
    unit_id = $2,
    position = $3,
    title = $4,
    status = CASE WHEN EXISTS (SELECT 1 FROM edit) THEN 'to_review' ELSE lessons.status END,
    validated_by = CASE WHEN EXISTS (SELECT 1 FROM edit) THEN NULL ELSE lessons.validated_by END,
    validated_at = CASE WHEN EXISTS (SELECT 1 FROM edit) THEN NULL ELSE lessons.validated_at END,
    updated_by = $5,
    updated_at = now()
WHERE
    id = $1
    AND (lessons.unit_id, lessons.position, lessons.title)
    IS DISTINCT FROM ($2::uuid, $3::integer, $4::text);
