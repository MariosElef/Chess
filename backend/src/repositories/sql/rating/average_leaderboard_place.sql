-- Where player $1 stands on the average leaderboard, ranked as average_leaderboard.sql ranks it.
WITH averaged AS (
    SELECT
        ratings.user_id,
        avg(ratings.rating) AS rating,
        avg(ratings.deviation) AS deviation,
        avg(ratings.volatility) AS volatility,
        sum(ratings.games_played)::integer AS games_played
    FROM play.ratings
    WHERE ratings.games_played > 0
    GROUP BY ratings.user_id
),

ranked AS (
    SELECT
        averaged.user_id,
        users.username,
        users.avatar_hash AS avatar,
        averaged.rating,
        averaged.deviation,
        averaged.volatility,
        averaged.games_played,
        rank() OVER (ORDER BY round(averaged.rating) DESC) AS place
    FROM averaged
    INNER JOIN account.users ON users.id = averaged.user_id
    WHERE
        users.deleted_at IS NULL
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
WHERE ranked.user_id = $1
