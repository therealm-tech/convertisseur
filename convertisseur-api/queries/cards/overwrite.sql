-- The content files' text replaces the card's: the files are reviewed in
-- pull requests, so it is validated by the author of the sync, and its
-- review note, answered by the change, is cleared.
UPDATE cards
SET
    arabic = $2,
    transliteration = $3,
    meaning_fr = $4,
    variety = $5,
    initial_form = $7,
    medial_form = $8,
    final_form = $9,
    status = 'validated',
    validated_by = $6,
    validated_at = now(),
    review_note = NULL,
    updated_by = $6,
    updated_at = now()
WHERE id = $1;
