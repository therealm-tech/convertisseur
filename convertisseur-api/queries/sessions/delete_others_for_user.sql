DELETE FROM sessions
WHERE user_id = $1 AND token_hash <> $2;
