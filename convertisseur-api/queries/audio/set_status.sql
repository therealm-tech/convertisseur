-- $3 is the validating account, NULL when sending back to review.
UPDATE audio
SET
    status = $2,
    validated_by = CASE WHEN audio.status = $2 THEN audio.validated_by ELSE $3 END,
    validated_at = CASE
        WHEN audio.status = $2 THEN audio.validated_at
        WHEN $3 IS NOT NULL THEN now()
    END
WHERE id = $1
RETURNING status, validated_at, validated_by;
