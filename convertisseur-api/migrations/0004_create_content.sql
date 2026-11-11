CREATE TYPE content_track AS ENUM ('arabe', 'islam');

-- Content to review is never shown to learners.
CREATE TYPE content_status AS ENUM ('to_review', 'validated');

CREATE TYPE arabic_variety AS ENUM ('classique', 'darija');

CREATE TYPE content_kind AS ENUM ('unit', 'lesson', 'card');

-- Positions are 1-based and dense among siblings: the units of a track, the
-- lessons of a unit, the cards of a lesson. Moving an item shifts its
-- siblings within one transaction, hence the deferred uniqueness.
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
    CONSTRAINT units_position_key UNIQUE (track, position) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT units_validated_check CHECK ((status = 'validated') = (validated_at IS NOT NULL))
);

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
    CONSTRAINT lessons_position_key UNIQUE (unit_id, position) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT lessons_validated_check CHECK (
        (status = 'validated') = (validated_at IS NOT NULL)
    )
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
    CONSTRAINT cards_position_key UNIQUE (lesson_id, position) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT cards_validated_check CHECK ((status = 'validated') = (validated_at IS NOT NULL))
);

-- What text search compares: lowercase, without the Arabic short vowels and
-- other diacritics (U+064B to U+065F, U+0670) nor the tatweel (U+0640), so
-- that unvocalised input finds vocalised text.
CREATE FUNCTION content_search_key(value text) RETURNS text
LANGUAGE sql IMMUTABLE PARALLEL SAFE
AS $$
    SELECT lower(regexp_replace(value, '[ً-ٰٟـ]', '', 'g'))
$$;

-- Every unit, lesson and card, as the review list shows them.
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
    units.title AS search_text
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
    lessons.title AS search_text
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
    concat_ws(' ', cards.arabic, cards.transliteration, cards.meaning_fr) AS search_text
FROM cards
INNER JOIN lessons ON cards.lesson_id = lessons.id
INNER JOIN units ON lessons.unit_id = units.id;
