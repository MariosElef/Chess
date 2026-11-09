-- A rating set outright, with how many results it stands for: for the puzzle categories, whose
-- results need puzzles a fresh server may not have.
INSERT INTO play.ratings (user_id, category_id, rating, deviation, volatility, games_played)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT (user_id, category_id) DO UPDATE SET
    rating = excluded.rating,
    deviation = excluded.deviation,
    volatility = excluded.volatility,
    games_played = excluded.games_played,
    updated_at = now()
