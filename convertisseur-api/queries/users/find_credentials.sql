-- A username never holds an `@` but the configured super admin's may: when
-- it equals another account's email, the username wins.
SELECT
    id,
    username,
    email,
    role,
    must_change_password,
    password_hash
FROM users
WHERE lower(username) = lower($1) OR lower(email) = lower($1)
ORDER BY lower(username) = lower($1) DESC
LIMIT 1;
