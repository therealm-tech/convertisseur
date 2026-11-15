-- The review state of the recordings of the cards among $1 that hold one,
-- in their lesson's order.
SELECT
    cards.id AS card_id,
    audio.id,
    audio.status,
    audio.validated_at,
    audio.validated_by,
    audio.review_note
FROM cards
INNER JOIN audio ON cards.audio_id = audio.id
WHERE cards.id = any($1)
ORDER BY cards.position ASC;
