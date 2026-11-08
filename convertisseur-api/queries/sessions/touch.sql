UPDATE sessions
SET expires_at = now() + make_interval(secs => $2)
FROM users
WHERE
    sessions.token_hash = $1
    AND sessions.expires_at > now()
    AND users.id = sessions.user_id
RETURNING users.id, users.username, users.role;
