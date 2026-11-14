CREATE TYPE user_role AS ENUM ('user', 'admin', 'super_admin');

CREATE TABLE users (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    username text NOT NULL CHECK (username <> ''),
    password_hash text NOT NULL,
    role user_role NOT NULL DEFAULT 'user',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    -- Set when an admin resets the password; the account can do nothing but
    -- change it until then.
    must_change_password boolean NOT NULL DEFAULT FALSE,
    -- The last time one of the account's sessions was used. It stays on the
    -- account once the sessions are closed.
    last_active_at timestamptz
);

CREATE UNIQUE INDEX users_username_key ON users (lower(username));

-- At most one super admin; the start-up bootstrap makes it exactly one.
CREATE UNIQUE INDEX users_single_super_admin_key ON users (role)
WHERE role = 'super_admin';

-- The browser holds the token; only its SHA-256 is stored, so a leaked table
-- cannot be replayed as cookies.
CREATE TABLE sessions (
    token_hash bytea PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL
);

CREATE INDEX sessions_user_id_idx ON sessions (user_id);

CREATE INDEX sessions_expires_at_idx ON sessions (expires_at);

CREATE TYPE content_track AS ENUM ('arabe', 'islam');

-- Content to review is never shown to learners.
CREATE TYPE content_status AS ENUM ('to_review', 'validated');

CREATE TYPE arabic_variety AS ENUM ('classique', 'darija');

CREATE TYPE content_kind AS ENUM ('unit', 'lesson', 'card');

-- Positions are 1-based and dense among siblings: the units of a track, the
-- lessons of a unit, the cards of a lesson. Moving an item shifts its
-- siblings within one transaction, hence the deferred uniqueness.
--
-- import_key is the stable key of a row imported from the content files, by
-- which the content sync finds it again; NULL for what staff write in the
-- admin area. review_note is what a reviewer reports about an item,
-- e.g. a wrong vowel; NULL when there is nothing to report.
CREATE TABLE units (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    track content_track NOT NULL,
    position integer NOT NULL CHECK (position >= 1),
    title text NOT NULL CHECK (title <> ''),
    status content_status NOT NULL DEFAULT 'to_review',
    created_by uuid REFERENCES users (id) ON DELETE SET NULL,
    updated_by uuid REFERENCES users (id) ON DELETE SET NULL,
    validated_by uuid REFERENCES users (id) ON DELETE SET NULL,
    validated_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    import_key text UNIQUE,
    review_note text CHECK (review_note <> '' AND char_length(review_note) <= 2000),
    CONSTRAINT units_position_key UNIQUE (track, position) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT units_validated_check CHECK ((status = 'validated') = (validated_at IS NOT NULL))
);

-- An uploaded recording; its bytes live in the object store under
-- object_key.
CREATE TABLE audio (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    object_key text NOT NULL UNIQUE,
    content_type text NOT NULL,
    size_bytes bigint NOT NULL CHECK (size_bytes > 0),
    duration_ms integer CHECK (duration_ms >= 0),
    created_by uuid REFERENCES users (id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

-- A lesson can carry a recording too, like a card.
CREATE TABLE lessons (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    unit_id uuid NOT NULL REFERENCES units (id),
    position integer NOT NULL CHECK (position >= 1),
    title text NOT NULL CHECK (title <> ''),
    status content_status NOT NULL DEFAULT 'to_review',
    created_by uuid REFERENCES users (id) ON DELETE SET NULL,
    updated_by uuid REFERENCES users (id) ON DELETE SET NULL,
    validated_by uuid REFERENCES users (id) ON DELETE SET NULL,
    validated_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    audio_id uuid UNIQUE REFERENCES audio (id),
    import_key text UNIQUE,
    review_note text CHECK (review_note <> '' AND char_length(review_note) <= 2000),
    CONSTRAINT lessons_position_key UNIQUE (unit_id, position) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT lessons_validated_check CHECK (
        (status = 'validated') = (validated_at IS NOT NULL)
    )
);

-- The initial, medial and final forms are the shapes of a letter within a
-- word, beside its isolated `arabic`: all three for a letter that joins the
-- next one, the final one alone for a letter that does not, none for any
-- other card.
CREATE TABLE cards (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    lesson_id uuid NOT NULL REFERENCES lessons (id),
    position integer NOT NULL CHECK (position >= 1),
    arabic text NOT NULL CHECK (arabic <> ''),
    transliteration text,
    meaning_fr text,
    variety arabic_variety NOT NULL,
    audio_id uuid UNIQUE REFERENCES audio (id),
    status content_status NOT NULL DEFAULT 'to_review',
    created_by uuid REFERENCES users (id) ON DELETE SET NULL,
    updated_by uuid REFERENCES users (id) ON DELETE SET NULL,
    validated_by uuid REFERENCES users (id) ON DELETE SET NULL,
    validated_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    import_key text UNIQUE,
    review_note text CHECK (review_note <> '' AND char_length(review_note) <= 2000),
    initial_form text,
    medial_form text,
    final_form text,
    CONSTRAINT cards_position_key UNIQUE (lesson_id, position) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT cards_validated_check CHECK ((status = 'validated') = (validated_at IS NOT NULL)),
    CONSTRAINT cards_forms_check CHECK (
        (initial_form IS NULL) = (medial_form IS NULL)
        AND (initial_form IS NULL OR final_form IS NOT NULL)
    )
);

-- What text search compares: lowercase, without the Arabic short vowels and
-- other diacritics (U+064B to U+065F, U+0670) nor the tatweel (U+0640), so
-- that unvocalised input finds vocalised text.
CREATE FUNCTION content_search_key(value text) RETURNS text
LANGUAGE sql IMMUTABLE PARALLEL SAFE
AS $$
    SELECT lower(regexp_replace(value, '[ً-ٰٟـ]', '', 'g'))
$$;

-- Every unit, lesson and card, as the review list shows them, linked to its
-- unit and lesson; the text search also matches the parent's title.
CREATE VIEW content_items AS
SELECT
    'unit'::content_kind AS kind,
    units.id,
    units.track,
    units.title,
    NULL::text AS transliteration,
    NULL::text AS parent_title,
    units.status,
    units.updated_at,
    units.title AS search_text,
    units.import_key,
    units.review_note,
    NULL::uuid AS unit_id,
    NULL::uuid AS lesson_id
FROM units
UNION ALL
SELECT
    'lesson'::content_kind AS kind,
    lessons.id,
    units.track,
    lessons.title,
    NULL::text AS transliteration,
    units.title AS parent_title,
    lessons.status,
    lessons.updated_at,
    concat_ws(' ', lessons.title, units.title) AS search_text,
    lessons.import_key,
    lessons.review_note,
    units.id AS unit_id,
    NULL::uuid AS lesson_id
FROM lessons
INNER JOIN units ON lessons.unit_id = units.id
UNION ALL
SELECT
    'card'::content_kind AS kind,
    cards.id,
    units.track,
    cards.arabic AS title,
    cards.transliteration,
    lessons.title AS parent_title,
    cards.status,
    cards.updated_at,
    concat_ws(
        ' ', cards.arabic, cards.transliteration, cards.meaning_fr, lessons.title
    ) AS search_text,
    cards.import_key,
    cards.review_note,
    units.id AS unit_id,
    lessons.id AS lesson_id
FROM cards
INNER JOIN lessons ON cards.lesson_id = lessons.id
INNER JOIN units ON lessons.unit_id = units.id;

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
