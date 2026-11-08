UPDATE users
SET
    password_hash = $2,
    must_change_password = true,
    updated_at = now()
WHERE id = $1;
