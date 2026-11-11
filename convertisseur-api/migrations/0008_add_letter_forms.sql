-- The shapes of a letter within a word, beside its isolated `arabic`: all
-- three for a letter that joins the next one, the final one alone for a
-- letter that does not, none for any other card.
ALTER TABLE cards
ADD COLUMN initial_form text,
ADD COLUMN medial_form text,
ADD COLUMN final_form text,
ADD CONSTRAINT cards_forms_check CHECK (
    (initial_form IS NULL) = (medial_form IS NULL)
    AND (initial_form IS NULL OR final_form IS NOT NULL)
);
