UPDATE users
SET
    role = 'admin',
    updated_at = now()
WHERE role = 'super_admin' AND lower(username) <> lower($1);
