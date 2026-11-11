-- $3 is the validating account, NULL when sending back to review.
UPDATE units
SET
    status = $2,
    validated_by = CASE WHEN units.status = $2 THEN units.validated_by ELSE $3 END,
    validated_at = CASE
        WHEN units.status = $2 THEN units.validated_at
        WHEN $3 IS NOT NULL THEN now()
    END
WHERE id = $1
RETURNING status, validated_at, validated_by;
