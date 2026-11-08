-- Exactly one of the game, the puzzle attempt and the battle is given: what moved the rating.
-- A closed account keeps no history.
INSERT INTO play.rating_history (
    id, user_id, category_id, game_id, puzzle_attempt_id, battle_id, rating, deviation,
    volatility
)
SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9
FROM account.users
WHERE users.id = $2 AND users.deleted_at IS NULL
FOR SHARE
