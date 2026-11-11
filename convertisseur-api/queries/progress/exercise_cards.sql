-- What decides whether each visible lesson of the track, or of every track
-- when $1 is NULL, has an exercise: its visible cards.
SELECT
    cards.lesson_id,
    cards.arabic,
    cards.meaning_fr,
    cards.final_form IS NOT NULL AS has_forms,
    cards.audio_id IS NOT NULL AS has_audio
FROM cards
INNER JOIN lessons ON cards.lesson_id = lessons.id
INNER JOIN units ON lessons.unit_id = units.id
WHERE
    cards.status = 'validated'
    AND lessons.status = 'validated'
    AND units.status = 'validated'
    AND ($1::content_track IS NULL OR units.track = $1);
