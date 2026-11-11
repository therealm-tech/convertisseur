SELECT
    lesson_id AS parent,
    position,
    import_key IS NOT NULL AS imported
FROM cards
WHERE id = $1
FOR UPDATE;
