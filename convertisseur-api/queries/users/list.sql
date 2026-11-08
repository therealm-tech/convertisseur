SELECT
    id,
    username,
    role,
    created_at,
    last_active_at
FROM users
WHERE
    ($1::text IS NULL OR strpos(lower(username), lower($1)) > 0)
    AND ($2::user_role IS NULL OR role = $2)
ORDER BY role DESC, lower(username) COLLATE "C" ASC, id ASC
LIMIT $3 OFFSET $4;
