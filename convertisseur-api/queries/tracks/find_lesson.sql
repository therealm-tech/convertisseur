-- A lesson of the track, when learners see it: it and its unit are
-- validated.
SELECT
    lessons.id,
    units.track,
    units.id AS unit_id,
    units.title AS unit_title,
    lessons.title,
    lessons.audio_id,
    audio.duration_ms AS audio_duration_ms
FROM lessons
INNER JOIN units ON lessons.unit_id = units.id
LEFT JOIN audio ON lessons.audio_id = audio.id
WHERE
    lessons.id = $1
    AND units.track = $2
    AND lessons.status = 'validated'
    AND units.status = 'validated';
