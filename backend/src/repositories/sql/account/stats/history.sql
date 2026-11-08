-- The player's latest $2 ratings in each category but $3 (not graphed), oldest first, for their
-- stats.
SELECT
    category,
    rating
FROM (
    SELECT
        rating_history.category_id AS category,
        rating_history.rating,
        rating_history.recorded_at,
        rating_history.id,
        row_number() OVER (
            PARTITION BY rating_history.category_id
            ORDER BY rating_history.recorded_at DESC, rating_history.id DESC
        ) AS newest_first
    FROM play.rating_history
    WHERE rating_history.user_id = $1 AND rating_history.category_id <> $3
)
WHERE newest_first <= $2
ORDER BY recorded_at, id
