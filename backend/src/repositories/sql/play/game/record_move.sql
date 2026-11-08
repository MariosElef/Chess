-- The move and the game's cached position and clocks, in one statement; any move clears an
-- open draw offer.
WITH moved AS (
    INSERT INTO play.game_moves (game_id, ply, uci, white_clock_ms, black_clock_ms)
    VALUES ($1, $2, $3, $5, $6)
)
UPDATE play.games
SET
    ply_count = $2,
    fen = $4,
    white_clock_ms = $5,
    black_clock_ms = $6,
    last_move_at = now(),
    draw_offer_by = NULL,
    updated_at = now()
WHERE games.id = $1
