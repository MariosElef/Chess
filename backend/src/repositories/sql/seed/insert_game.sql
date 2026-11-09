-- A made-up game, already finished: its moves follow in insert_moves.sql.
INSERT INTO play.games (
    id, white_id, black_id, category_id, initial_s, increment_s, rated, status, outcome, winner,
    ply_count, fen, white_clock_ms, black_clock_ms, last_move_at, started_at, finished_at,
    updated_at
)
VALUES (
    $1, $2, $3, $4, $5, $6, true, 'finished', $7, $8, $9, $10, $11, $12, $14, $13, $14, $14
)
