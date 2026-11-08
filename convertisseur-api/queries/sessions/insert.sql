WITH opened AS (
    INSERT INTO sessions (token_hash, user_id, expires_at)
    VALUES ($1, $2, now() + make_interval(secs => $3))
    RETURNING user_id
)

UPDATE users
SET last_active_at = now()
FROM opened
WHERE users.id = opened.user_id;
