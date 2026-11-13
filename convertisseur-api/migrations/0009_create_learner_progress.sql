-- What each account has done: the cards it has seen, and its best exercise
-- result per lesson. Deleting the account, the card or the lesson deletes
-- what it held.
CREATE TABLE card_views (
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    card_id uuid NOT NULL REFERENCES cards (id) ON DELETE CASCADE,
    first_seen_at timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, card_id)
);

CREATE INDEX card_views_card_id_idx ON card_views (card_id);

CREATE TABLE exercise_results (
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    lesson_id uuid NOT NULL REFERENCES lessons (id) ON DELETE CASCADE,
    best_first_try integer NOT NULL CHECK (best_first_try >= 0),
    best_total integer NOT NULL CHECK (best_total >= 1),
    best_at timestamptz NOT NULL,
    passed_at timestamptz,
    last_attempt_at timestamptz NOT NULL,
    PRIMARY KEY (user_id, lesson_id),
    CONSTRAINT exercise_results_best_check CHECK (best_first_try <= best_total)
);

CREATE INDEX exercise_results_lesson_id_idx ON exercise_results (lesson_id);
