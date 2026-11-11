-- Per lesson the account has seen a card of: how many of its visible cards
-- it has seen, and when it last saw any of them.
SELECT
    cards.lesson_id,
    count(*) FILTER (WHERE cards.status = 'validated') AS seen_card_count,
    max(card_views.last_seen_at) AS last_seen_at
FROM card_views
INNER JOIN cards ON card_views.card_id = cards.id
WHERE card_views.user_id = $1
GROUP BY cards.lesson_id;
