-- Where player $2 stands in category $1's leaderboard, ranked as leaderboard.sql ranks it.
WITH ranked AS (
    SELECT
        ratings.user_id,
        users.username,
        users.avatar_hash AS avatar,
        ratings.rating,
        ratings.deviation,
        ratings.volatility,
        ratings.games_played,
        rank() OVER (ORDER BY round(ratings.rating) DESC) AS place
    FROM play.ratings
    INNER JOIN account.users ON users.id = ratings.user_id
    WHERE
        ratings.category_id = $1
        AND ratings.games_played > 0
        AND users.deleted_at IS NULL
        AND users.guest_number IS NULL
)
SELECT
    ranked.user_id AS "user_id!",
    ranked.username AS "username!",
    ranked.avatar,
    ranked.rating AS "rating!",
    ranked.deviation AS "deviation!",
    ranked.volatility AS "volatility!",
    ranked.games_played AS "games_played!",
    ranked.place AS "place!"
FROM ranked
WHERE ranked.user_id = $2
