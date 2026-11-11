INSERT INTO cards (
    lesson_id,
    position,
    arabic,
    transliteration,
    meaning_fr,
    variety,
    import_key,
    created_by,
    updated_by
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
RETURNING id;
