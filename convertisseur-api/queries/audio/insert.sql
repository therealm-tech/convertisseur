INSERT INTO audio (id, object_key, content_type, size_bytes, duration_ms, created_by)
VALUES ($1, $2, $3, $4, $5, $6)
RETURNING id, content_type, size_bytes, duration_ms, created_at;
