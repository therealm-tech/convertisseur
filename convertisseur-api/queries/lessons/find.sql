SELECT
    lessons.id,
    lessons.unit_id,
    units.position AS unit_position,
    units.title AS unit_title,
    units.track,
    lessons.position,
    lessons.title,
    audio.id AS audio_id,
    audio.content_type AS audio_content_type,
    audio.size_bytes AS audio_size_bytes,
    audio.duration_ms AS audio_duration_ms,
    audio.created_at AS audio_created_at,
    lessons.status,
    lessons.created_at,
    lessons.created_by,
    lessons.updated_at,
    lessons.updated_by,
    lessons.validated_at,
    lessons.validated_by,
    lessons.import_key,
    lessons.review_note
FROM lessons
INNER JOIN units ON lessons.unit_id = units.id
LEFT JOIN audio ON lessons.audio_id = audio.id
WHERE lessons.id = $1;
