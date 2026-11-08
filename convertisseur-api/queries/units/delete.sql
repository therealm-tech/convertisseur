DELETE FROM units
WHERE id = $1
RETURNING track AS parent, position, NULL::uuid AS audio_id;
