-- Required at registration; accounts created before it, and the super admin
-- the bootstrap creates, have none until they set one.
ALTER TABLE users ADD COLUMN email text CHECK (email <> '');

CREATE UNIQUE INDEX users_email_key ON users (lower(email));
