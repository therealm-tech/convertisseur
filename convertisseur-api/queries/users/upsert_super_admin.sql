INSERT INTO users (username, password_hash, role)
VALUES ($1, $2, 'super_admin')
ON CONFLICT ((lower(username))) DO UPDATE
    SET
        password_hash = excluded.password_hash,
        role = excluded.role,
        updated_at = now()
RETURNING id;
