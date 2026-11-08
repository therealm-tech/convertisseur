-- $3 is the validating account, NULL when sending back to review.
UPDATE lessons
SET
    status = $2,
    validated_by = CASE WHEN lessons.status = $2 THEN lessons.validated_by ELSE $3 END,
    validated_at = CASE
        WHEN lessons.status = $2 THEN lessons.validated_at
        WHEN $3 IS NOT NULL THEN now()
    END
WHERE id = $1
RETURNING status, validated_at, validated_by;
