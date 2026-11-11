SELECT
    id,
    lesson_id AS parent,
    position,
    import_key,
    arabic,
    transliteration,
    meaning_fr,
    variety,
    review_note
FROM cards;
