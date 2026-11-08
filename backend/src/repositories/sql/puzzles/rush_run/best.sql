SELECT max(rush_runs.score) AS best
FROM puzzle.rush_runs
WHERE rush_runs.user_id = $1 AND rush_runs.kind = $2
