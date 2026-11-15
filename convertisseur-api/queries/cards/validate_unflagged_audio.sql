-- Validates the recordings of the cards among $2 of the lesson $1 that are
-- to review without a review note, by the account $3; flagged and
-- validated recordings stay as they are.
UPDATE audio
SET
    status = 'validated',
    validated_by = $3,
    validated_at = now()
FROM cards
WHERE
    cards.audio_id = audio.id
    AND cards.lesson_id = $1
    AND cards.id = any($2)
    AND audio.status = 'to_review'
    AND audio.review_note IS NULL;
