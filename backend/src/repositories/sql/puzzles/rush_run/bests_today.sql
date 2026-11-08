SELECT
    rush_runs.kind,
    max(rush_runs.score) AS "best!"
FROM puzzle.rush_runs
WHERE rush_runs.user_id = $1 AND rush_runs.finished_at >= date_trunc('day', now())
GROUP BY rush_runs.kind
