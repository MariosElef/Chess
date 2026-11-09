-- Every player's average rating over the categories they have a rated result in, best first,
-- $1 of them from place $2 on: open accounts, not guests. Equal averages as shown share a place,
-- listed by name; the deviation and volatility are averaged alike, the results added up.
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
ORDER BY ranked.place, ranked.username
LIMIT $1 OFFSET $2
