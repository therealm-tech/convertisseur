UPDATE users
SET
    email = $2,
    updated_at = now()
WHERE id = $1;
