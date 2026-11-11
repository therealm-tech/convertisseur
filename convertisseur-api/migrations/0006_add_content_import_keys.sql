-- The stable key of a row imported from the content files, by which
-- `import-content` knows it already exists; NULL for what staff write in the
-- admin area.
ALTER TABLE units ADD COLUMN import_key text UNIQUE;
ALTER TABLE lessons ADD COLUMN import_key text UNIQUE;
ALTER TABLE cards ADD COLUMN import_key text UNIQUE;
