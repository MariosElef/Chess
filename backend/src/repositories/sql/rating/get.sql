SELECT
    ratings.rating,
    ratings.deviation,
    ratings.volatility
FROM play.ratings
WHERE ratings.user_id = $1 AND ratings.category_id = $2
