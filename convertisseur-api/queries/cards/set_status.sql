-- $3 is the validating account, NULL when sending back to review.
UPDATE cards
SET
    status = $2,
    validated_by = CASE WHEN cards.status = $2 THEN cards.validated_by ELSE $3 END,
    validated_at = CASE
        WHEN cards.status = $2 THEN cards.validated_at
        WHEN $3 IS NOT NULL THEN now()
    END
WHERE id = $1
RETURNING status, validated_at, validated_by;
