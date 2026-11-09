-- The game $1, if $2 played in it and it finished in the last ten minutes: a rematch replays it.
SELECT
    games.white_id,
    games.black_id,
    games.initial_s,
    games.increment_s,
    games.rated
FROM play.games
WHERE
    games.id = $1
    AND (games.white_id = $2 OR games.black_id = $2)
    AND games.status = 'finished'
    AND games.finished_at > now() - interval '10 minutes'
