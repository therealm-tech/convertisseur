SELECT exists(
    SELECT 1
    FROM lessons
    WHERE unit_id = $1
);
