UPDATE users
SET
    password_hash = $3,
    must_change_password = false,
    updated_at = now()
WHERE id = $1 AND password_hash = $2;
