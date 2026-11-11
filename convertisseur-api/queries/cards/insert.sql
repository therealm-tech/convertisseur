INSERT INTO cards (
    lesson_id,
    position,
    arabic,
    transliteration,
    meaning_fr,
    variety,
    initial_form,
    medial_form,
    final_form,
    import_key,
    created_by,
    updated_by
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11)
RETURNING id;
