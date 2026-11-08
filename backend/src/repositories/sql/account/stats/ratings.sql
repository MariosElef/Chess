SELECT
    rating_categories.id AS "category!",
    ratings.rating AS "rating?",
    ratings.deviation AS "deviation?",
    ratings.volatility AS "volatility?",
    (
        SELECT max(rating_history.rating)
        FROM play.rating_history
        WHERE rating_history.user_id = $1 AND rating_history.category_id = rating_categories.id
    ) AS best
FROM play.rating_categories
LEFT JOIN play.ratings
    ON ratings.category_id = rating_categories.id AND ratings.user_id = $1
ORDER BY rating_categories.sort_order
