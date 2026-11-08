INSERT INTO play.games (
    id, white_id, black_id, category_id, initial_s, increment_s, rated, fen, white_clock_ms,
    black_clock_ms
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $5 * 1000, $5 * 1000)
