-- How many lessons match the filters of review/lessons.sql, across all
-- pages.
SELECT count(*)
FROM lessons
INNER JOIN units ON lessons.unit_id = units.id
WHERE
    ($1::uuid IS NULL OR lessons.unit_id = $1)
    AND (
        $2::boolean IS NOT TRUE
        OR lessons.status = 'to_review'
        OR units.status = 'to_review'
        OR EXISTS (
            SELECT 1
            FROM cards AS pending
            WHERE pending.lesson_id = lessons.id AND pending.status = 'to_review'
        )
    )
    AND (
        $3::boolean IS NOT TRUE
        OR EXISTS (
            SELECT 1
            FROM cards AS silent
            WHERE silent.lesson_id = lessons.id AND silent.audio_id IS NULL
        )
    );
