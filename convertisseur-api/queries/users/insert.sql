INSERT INTO users (username, password_hash)
VALUES ($1, $2)
RETURNING id, username, role, must_change_password;
