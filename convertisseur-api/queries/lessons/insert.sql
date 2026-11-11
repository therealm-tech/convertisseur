INSERT INTO lessons (unit_id, position, title, import_key, created_by, updated_by)
VALUES ($1, $2, $3, $4, $5, $5)
RETURNING id;
