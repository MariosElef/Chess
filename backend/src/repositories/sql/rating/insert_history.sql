-- Exactly one of the game and the puzzle attempt is given: what moved the rating.
INSERT INTO play.rating_history (
    id, user_id, category_id, game_id, puzzle_attempt_id, rating, deviation, volatility
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
