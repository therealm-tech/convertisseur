SELECT
    lesson_id,
    best_first_try,
    best_total,
    best_at,
    passed_at,
    last_attempt_at
FROM exercise_results
WHERE user_id = $1;
