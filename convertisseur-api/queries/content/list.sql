SELECT
    kind,
    id,
    track,
    title,
    transliteration,
    parent_title,
    status,
    updated_at,
    import_key,
    review_note
FROM content_items
WHERE
    ($1::content_status IS NULL OR status = $1)
    AND ($2::content_track IS NULL OR track = $2)
    AND ($3::content_kind IS NULL OR kind = $3)
    AND ($4::text IS NULL OR strpos(content_search_key(search_text), content_search_key($4)) > 0)
    AND ($5::boolean IS NULL OR (review_note IS NOT NULL) = $5)
ORDER BY updated_at DESC, id ASC
LIMIT $6 OFFSET $7;
