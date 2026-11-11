-- One page of the lessons matching the filters, in the order of the tracks,
-- with their unit and the review and audio state of their cards. A card is
-- flagged when it is to review with a review note. $1 the unit, $2 pending
-- review only (the lesson, its unit or a card is to review), $3 lessons
-- with a card without audio only.
SELECT
    lessons.id,
    lessons.position,
    lessons.title,
    lessons.status,
    lessons.review_note,
    units.id AS unit_id,
    units.track AS unit_track,
    units.position AS unit_position,
    units.title AS unit_title,
    units.status AS unit_status,
    units.review_note AS unit_review_note,
    lessons.audio_id IS NOT NULL AS has_audio,
    count(cards.id) FILTER (WHERE cards.status = 'validated') AS validated_cards,
    count(cards.id) FILTER (
        WHERE cards.status = 'to_review' AND cards.review_note IS NOT NULL
    ) AS flagged_cards,
    count(cards.id) FILTER (
        WHERE cards.status = 'to_review' AND cards.review_note IS NULL
    ) AS cards_to_review,
    count(cards.id) FILTER (WHERE cards.audio_id IS NULL) AS cards_without_audio
FROM lessons
INNER JOIN units ON lessons.unit_id = units.id
LEFT JOIN cards ON lessons.id = cards.lesson_id
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
    )
GROUP BY lessons.id, units.id
ORDER BY units.track ASC, units.position ASC, lessons.position ASC
LIMIT $4 OFFSET $5;
