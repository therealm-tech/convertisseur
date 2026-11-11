INSERT INTO lessons (unit_id, position, title, created_by, updated_by)
VALUES ($1, $2, $3, $4, $4)
RETURNING id;
