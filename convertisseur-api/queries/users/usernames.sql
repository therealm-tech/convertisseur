SELECT
    id,
    username
FROM users
WHERE id = any($1);
