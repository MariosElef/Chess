SELECT
    count(*) AS "attempted!",
    count(*) FILTER (WHERE puzzle_attempts.solved) AS "solved!"
FROM puzzle.puzzle_attempts
WHERE puzzle_attempts.user_id = $1
