INSERT INTO users (username, password_hash)
VALUES ($1, $2)
RETURNING id, username, role;
