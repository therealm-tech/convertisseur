UPDATE users
SET
    role = $2,
    updated_at = now()
WHERE id = $1;
