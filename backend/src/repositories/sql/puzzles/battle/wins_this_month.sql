SELECT count(*) AS "wins!"
FROM puzzle.battles
WHERE battles.winner_id = $1 AND battles.finished_at >= date_trunc('month', now())
