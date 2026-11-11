SELECT exists(
    SELECT 1
    FROM cards
    WHERE lesson_id = $1
);
