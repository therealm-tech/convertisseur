-- Set when an admin resets the password; the account can do nothing but
-- change it until then.
ALTER TABLE users ADD COLUMN must_change_password boolean NOT NULL DEFAULT false;

-- The last time one of the account's sessions was used. It stays on the
-- account once the sessions are closed.
ALTER TABLE users ADD COLUMN last_active_at timestamptz;
