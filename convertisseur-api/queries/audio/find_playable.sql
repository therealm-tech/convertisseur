-- A recording, and whether learners see what holds it: a card validated with
-- its lesson and unit, or a lesson validated with its unit. FALSE when
-- nothing holds it.
SELECT
    audio.object_key,
    audio.content_type,
    audio.size_bytes,
    coalesce(
        cards.status = 'validated'
        AND card_lessons.status = 'validated'
        AND card_units.status = 'validated',
        lessons.status = 'validated'
        AND lesson_units.status = 'validated',
        FALSE
    ) AS visible
FROM audio
LEFT JOIN cards ON audio.id = cards.audio_id
LEFT JOIN lessons AS card_lessons ON cards.lesson_id = card_lessons.id
LEFT JOIN units AS card_units ON card_lessons.unit_id = card_units.id
LEFT JOIN lessons ON audio.id = lessons.audio_id
LEFT JOIN units AS lesson_units ON lessons.unit_id = lesson_units.id
WHERE audio.id = $1;
