-- A recording, with the review status of the card or lesson that holds it;
-- NULL when none does.
SELECT
    audio.object_key,
    audio.content_type,
    audio.size_bytes,
    coalesce(cards.status, lessons.status) AS status
FROM audio
LEFT JOIN cards ON audio.id = cards.audio_id
LEFT JOIN lessons ON audio.id = lessons.audio_id
WHERE audio.id = $1;
