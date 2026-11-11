-- How many items match the track, kind and text filters, per status and
-- per presence of a review note; each count of the page leaves out its own
-- filter.
SELECT
    status,
    review_note IS NOT NULL AS noted,
    count(*) AS items
FROM content_items
WHERE
    ($1::content_track IS NULL OR track = $1)
    AND ($2::content_kind IS NULL OR kind = $2)
    AND ($3::text IS NULL OR strpos(content_search_key(search_text), content_search_key($3)) > 0)
GROUP BY status, noted;
