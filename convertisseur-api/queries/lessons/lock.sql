SELECT
    unit_id AS parent,
    position,
    import_key IS NOT NULL AS imported
FROM lessons
WHERE id = $1
FOR UPDATE;
