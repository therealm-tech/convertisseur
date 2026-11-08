SELECT
    id,
    username,
    role,
    must_change_password,
    password_hash
FROM users
WHERE id = $1;
