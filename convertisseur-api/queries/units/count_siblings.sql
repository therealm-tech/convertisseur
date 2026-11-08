SELECT count(*)
FROM units
WHERE track = $1 AND id IS DISTINCT FROM $2;
