SELECT
    games.category_id AS category,
    count(*) AS "games!",
    count(*) FILTER (
        WHERE (games.winner = 'white' AND games.white_id = $1)
        OR (games.winner = 'black' AND games.black_id = $1)
    ) AS "wins!",
    count(*) FILTER (WHERE games.winner IS NULL) AS "draws!"
FROM play.games
WHERE games.status = 'finished' AND (games.white_id = $1 OR games.black_id = $1)
GROUP BY games.category_id
