SELECT exists(
    SELECT 1
    FROM lessons
    WHERE id = $1
);
