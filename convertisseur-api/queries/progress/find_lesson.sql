-- The track of a lesson learners see.
SELECT units.track
FROM lessons
INNER JOIN units ON lessons.unit_id = units.id
WHERE
    lessons.id = $1
    AND lessons.status = 'validated'
    AND units.status = 'validated';
