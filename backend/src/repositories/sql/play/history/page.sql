-- Up to $3 of the player $1's finished games before the game $2 (all, without it), the latest
-- first; ids are uuid v7, so they sort by when the games began. Each side reads its own index.
SELECT
    games.id,
    games.white_id,
    (SELECT users.username FROM account.users WHERE users.id = games.white_id) AS "white_name!",
    (SELECT users.username FROM account.users WHERE users.id = games.black_id) AS "black_name!",
    games.category_id AS category,
    games.initial_s,
    games.increment_s,
    games.rated,
    games.outcome AS "outcome!",
    games.winner,
    games.ply_count,
    floor(extract(EPOCH FROM now() - games.finished_at))::bigint AS "ago_s!"
FROM play.games
WHERE games.id IN (
    (
        SELECT games.id
        FROM play.games
        WHERE
            games.white_id = $1
            AND games.status = 'finished'
            AND ($2::uuid IS NULL OR games.id < $2)
        ORDER BY games.id DESC
        LIMIT $3
    )
    UNION ALL
    (
        SELECT games.id
        FROM play.games
        WHERE
            games.black_id = $1
            AND games.status = 'finished'
            AND ($2::uuid IS NULL OR games.id < $2)
        ORDER BY games.id DESC
        LIMIT $3
    )
)
ORDER BY games.id DESC
LIMIT $3
