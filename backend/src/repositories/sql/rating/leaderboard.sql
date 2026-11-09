-- The players rated in category $1, best first, $2 of them from place $3 on: open accounts, not
-- guests, that have a rated result in it. Equal ratings as shown share a place, listed by name.
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
ORDER BY ranked.place, ranked.username
LIMIT $2 OFFSET $3
