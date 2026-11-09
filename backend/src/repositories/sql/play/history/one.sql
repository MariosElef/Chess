-- The finished game $2, if the player $1 played in it.
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
WHERE
    games.id = $2
    AND (games.white_id = $1 OR games.black_id = $1)
    AND games.status = 'finished'
