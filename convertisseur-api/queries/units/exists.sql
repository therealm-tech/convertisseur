SELECT exists(
    SELECT 1
    FROM units
    WHERE id = $1
);
