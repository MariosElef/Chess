-- A closed account keeps no rating: a game or battle ending after it closed records nothing, and
-- one ending while it closes waits for it.
INSERT INTO play.ratings (user_id, category_id, rating, deviation, volatility, games_played)
SELECT $1, $2, $3, $4, $5, 1
FROM account.users
WHERE users.id = $1 AND users.deleted_at IS NULL
FOR SHARE
ON CONFLICT (user_id, category_id) DO UPDATE SET
    rating = excluded.rating,
    deviation = excluded.deviation,
    volatility = excluded.volatility,
    games_played = ratings.games_played + 1,
    updated_at = now()
