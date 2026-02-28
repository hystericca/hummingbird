DELETE FROM playlist_item
WHERE track_id IN (
    SELECT id FROM track WHERE location = $1
);
