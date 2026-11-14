SELECT
    id,
    username,
    email,
    role,
    must_change_password
FROM users
WHERE id = $1
FOR UPDATE;
