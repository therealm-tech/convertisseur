-- One page of the lessons matching the filters, in the order of the tracks,
-- with their unit and the review and audio state of their cards. A card is
-- flagged when it is to review with a review note. $1 the unit, $2 pending
-- review only (the lesson, its unit or a card is to review), $3 lessons
-- with a card without audio only, $4 lessons with a recording to review
-- only, theirs or a card's. A recording is flagged like a card.
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
    lesson_audio.status AS audio_status,
    lessons.audio_id IS NOT NULL AS has_audio,
    count(cards.id) FILTER (WHERE cards.status = 'validated') AS validated_cards,
    count(cards.id) FILTER (
        WHERE cards.status = 'to_review' AND cards.review_note IS NOT NULL
    ) AS flagged_cards,
    count(cards.id) FILTER (
        WHERE cards.status = 'to_review' AND cards.review_note IS NULL
    ) AS cards_to_review,
    count(cards.id) FILTER (WHERE cards.audio_id IS NULL) AS cards_without_audio,
    count(card_audio.id) FILTER (WHERE card_audio.status = 'validated') AS validated_recordings,
    count(card_audio.id) FILTER (
        WHERE card_audio.status = 'to_review' AND card_audio.review_note IS NOT NULL
    ) AS flagged_recordings,
    count(card_audio.id) FILTER (
        WHERE card_audio.status = 'to_review' AND card_audio.review_note IS NULL
    ) AS recordings_to_review
FROM lessons
INNER JOIN units ON lessons.unit_id = units.id
LEFT JOIN audio AS lesson_audio ON lessons.audio_id = lesson_audio.id
LEFT JOIN cards ON lessons.id = cards.lesson_id
LEFT JOIN audio AS card_audio ON cards.audio_id = card_audio.id
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
    AND (
        $4::boolean IS NOT TRUE
        OR EXISTS (
            SELECT 1
            FROM audio AS unheard
            LEFT JOIN cards AS holder ON unheard.id = holder.audio_id
            WHERE
                unheard.status = 'to_review'
                AND (holder.lesson_id = lessons.id OR unheard.id = lessons.audio_id)
        )
    )
GROUP BY lessons.id, units.id, lesson_audio.id
ORDER BY units.track ASC, units.position ASC, lessons.position ASC
LIMIT $5 OFFSET $6;
