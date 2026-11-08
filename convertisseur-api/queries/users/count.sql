SELECT count(*)
FROM users
WHERE
    ($1::text IS NULL OR strpos(lower(username), lower($1)) > 0)
    AND ($2::user_role IS NULL OR role = $2);
