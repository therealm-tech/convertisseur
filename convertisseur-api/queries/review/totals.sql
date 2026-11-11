-- What the review queue's header counts, whatever the filters: the units,
-- lessons and cards to review, and the cards without a recording.
SELECT
    (
        SELECT count(*)
        FROM content_items
        WHERE status = 'to_review'
    ) AS items_to_review,
    (
        SELECT count(*)
        FROM cards
        WHERE audio_id IS NULL
    ) AS cards_without_audio;
