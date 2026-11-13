-- Folds an attempt into the account's result for the lesson: the best
-- share of first-try answers stays, a tie keeping the earlier one, and the
-- first pass is never cleared.
INSERT INTO exercise_results AS results (
    user_id, lesson_id, best_first_try, best_total, best_at, passed_at, last_attempt_at
)
VALUES ($1, $2, $3, $4, now(), CASE WHEN $5 THEN now() END, now())
ON CONFLICT (user_id, lesson_id) DO UPDATE
    SET
        best_first_try = CASE
            WHEN
                excluded.best_first_try * results.best_total
                > results.best_first_try * excluded.best_total
                THEN excluded.best_first_try
            ELSE results.best_first_try
        END,
        best_total = CASE
            WHEN
                excluded.best_first_try * results.best_total
                > results.best_first_try * excluded.best_total
                THEN excluded.best_total
            ELSE results.best_total
        END,
        best_at = CASE
            WHEN
                excluded.best_first_try * results.best_total
                > results.best_first_try * excluded.best_total
                THEN excluded.best_at
            ELSE results.best_at
        END,
        passed_at = coalesce(results.passed_at, excluded.passed_at),
        last_attempt_at = excluded.last_attempt_at
RETURNING
    lesson_id,
    best_first_try,
    best_total,
    best_at,
    passed_at,
    last_attempt_at;
