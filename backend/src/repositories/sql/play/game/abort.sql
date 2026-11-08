UPDATE play.games
SET
    status = 'aborted',
    draw_offer_by = NULL,
    finished_at = now(),
    updated_at = now()
WHERE games.id = $1 AND games.status = 'ongoing'
