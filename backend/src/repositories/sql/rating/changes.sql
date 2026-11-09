-- The player $1's rating after each of the results $2 (games, puzzle attempts or battles) and
-- the one just before it in its category, 1500 for the first; ids are uuid v7, minted in order.
WITH results AS (
    SELECT
        rating_history.id,
        rating_history.category_id,
        rating_history.rating,
        coalesce(
            rating_history.game_id, rating_history.puzzle_attempt_id, rating_history.battle_id
        ) AS source_id
    FROM play.rating_history
    WHERE
        rating_history.user_id = $1
        AND coalesce(
            rating_history.game_id, rating_history.puzzle_attempt_id, rating_history.battle_id
        ) = ANY($2)
)

SELECT
    results.source_id AS "source_id!",
    results.rating AS "rating!",
    coalesce(
        (
            SELECT rating_history.rating
            FROM play.rating_history
            WHERE
                rating_history.user_id = $1
                AND rating_history.category_id = results.category_id
                AND rating_history.id < results.id
            ORDER BY rating_history.id DESC
            LIMIT 1
        ),
        1500
    ) AS "before!"
FROM results
