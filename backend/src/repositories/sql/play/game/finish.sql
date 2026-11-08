UPDATE play.games
SET
    status = 'finished',
    outcome = $2,
    winner = $3,
    white_clock_ms = $4,
    black_clock_ms = $5,
    draw_offer_by = NULL,
    finished_at = now(),
    updated_at = now()
WHERE games.id = $1 AND games.status = 'ongoing'
