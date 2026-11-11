SELECT
    cards.id,
    cards.lesson_id,
    lessons.title AS lesson_title,
    units.track,
    cards.position,
    cards.arabic,
    cards.transliteration,
    cards.meaning_fr,
    cards.variety,
    audio.id AS audio_id,
    audio.content_type AS audio_content_type,
    audio.size_bytes AS audio_size_bytes,
    audio.duration_ms AS audio_duration_ms,
    audio.created_at AS audio_created_at,
    cards.status,
    cards.created_at,
    cards.created_by,
    cards.updated_at,
    cards.updated_by,
    cards.validated_at,
    cards.validated_by,
    cards.import_key,
    cards.review_note
FROM cards
INNER JOIN lessons ON cards.lesson_id = lessons.id
INNER JOIN units ON lessons.unit_id = units.id
LEFT JOIN audio ON cards.audio_id = audio.id
WHERE cards.lesson_id = $1
ORDER BY cards.position ASC
LIMIT $2 OFFSET $3;
