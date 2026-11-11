SELECT
    track AS parent,
    position,
    import_key IS NOT NULL AS imported
FROM units
WHERE id = $1
FOR UPDATE;
