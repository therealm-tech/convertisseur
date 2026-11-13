INSERT INTO card_views (user_id, card_id)
VALUES ($1, $2)
ON CONFLICT (user_id, card_id) DO UPDATE SET last_seen_at = now();
