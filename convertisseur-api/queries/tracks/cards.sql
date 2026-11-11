-- The cards of a lesson learners see, in order: they, their lesson and its
-- unit are validated.
SELECT
    cards.id,
    cards.arabic,
    cards.transliteration,
    cards.meaning_fr,
    cards.variety,
    cards.initial_form,
    cards.medial_form,
    cards.final_form,
    cards.audio_id,
    audio.duration_ms AS audio_duration_ms
FROM cards
INNER JOIN lessons ON cards.lesson_id = lessons.id
INNER JOIN units ON lessons.unit_id = units.id
LEFT JOIN audio ON cards.audio_id = audio.id
WHERE
    cards.lesson_id = $1
    AND cards.status = 'validated'
    AND lessons.status = 'validated'
    AND units.status = 'validated'
ORDER BY cards.position ASC;
