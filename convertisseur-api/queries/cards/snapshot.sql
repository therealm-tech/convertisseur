SELECT
    id,
    lesson_id AS parent,
    position,
    import_key,
    arabic,
    transliteration,
    meaning_fr,
    variety,
    initial_form,
    medial_form,
    final_form,
    review_note
FROM cards;
