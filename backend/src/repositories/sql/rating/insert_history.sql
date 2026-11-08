-- A puzzle attempt's rating; a game's will name its game instead.
INSERT INTO play.rating_history (
    id, user_id, category_id, puzzle_attempt_id, rating, deviation, volatility
)
VALUES ($1, $2, $3, $4, $5, $6, $7)
