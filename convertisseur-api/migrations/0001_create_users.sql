CREATE TYPE user_role AS ENUM ('user', 'admin', 'super_admin');

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username TEXT NOT NULL CHECK (username <> ''),
    password_hash TEXT NOT NULL,
    role user_role NOT NULL DEFAULT 'user',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX users_username_key ON users (lower(username));

-- At most one super admin; the start-up bootstrap makes it exactly one.
CREATE UNIQUE INDEX users_single_super_admin_key ON users (role)
WHERE role = 'super_admin';
