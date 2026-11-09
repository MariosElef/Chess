-- Every move of game $1, numbered from 1, with both clocks after each.
INSERT INTO play.game_moves (game_id, ply, uci, white_clock_ms, black_clock_ms, played_at)
SELECT
    $1,
    moves.ply,
    moves.uci,
    moves.white_clock_ms,
    moves.black_clock_ms,
    moves.played_at
FROM unnest($2::text[], $3::integer[], $4::integer[], $5::timestamptz[])
    WITH ORDINALITY AS moves (uci, white_clock_ms, black_clock_ms, played_at, ply)
