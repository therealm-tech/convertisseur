SELECT card_views.card_id
FROM card_views
INNER JOIN cards ON card_views.card_id = cards.id
WHERE card_views.user_id = $1 AND cards.lesson_id = $2;
