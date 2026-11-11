WITH edit AS (
    SELECT id
    FROM cards
    WHERE
        id = $1
        AND (arabic, transliteration, meaning_fr, variety)
        IS DISTINCT FROM ($4::text, $5::text, $6::text, $7::arabic_variety)
)

UPDATE cards
SET
    lesson_id = $2,
    position = $3,
    arabic = $4,
    transliteration = $5,
    meaning_fr = $6,
    variety = $7,
    status = CASE WHEN EXISTS (SELECT 1 FROM edit) THEN 'to_review' ELSE cards.status END,
    validated_by = CASE WHEN EXISTS (SELECT 1 FROM edit) THEN NULL ELSE cards.validated_by END,
    validated_at = CASE WHEN EXISTS (SELECT 1 FROM edit) THEN NULL ELSE cards.validated_at END,
    updated_by = $8,
    updated_at = now()
WHERE
    id = $1
    AND (lesson_id, position, arabic, transliteration, meaning_fr, variety)
    IS DISTINCT FROM ($2::uuid, $3::integer, $4::text, $5::text, $6::text, $7::arabic_variety);
