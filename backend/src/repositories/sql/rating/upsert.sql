INSERT INTO play.ratings (user_id, category_id, rating, deviation, volatility, games_played)
VALUES ($1, $2, $3, $4, $5, 1)
ON CONFLICT (user_id, category_id) DO UPDATE SET
    rating = excluded.rating,
    deviation = excluded.deviation,
    volatility = excluded.volatility,
    games_played = ratings.games_played + 1,
    updated_at = now()
