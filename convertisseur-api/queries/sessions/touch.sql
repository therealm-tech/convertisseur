WITH touched AS (
    UPDATE sessions
    SET expires_at = now() + make_interval(secs => $2)
    WHERE token_hash = $1 AND expires_at > now()
    RETURNING user_id
)

UPDATE users
SET last_active_at = now()
FROM touched
WHERE users.id = touched.user_id
RETURNING
    users.id,
    users.username,
    users.email,
    users.role,
    users.must_change_password;
