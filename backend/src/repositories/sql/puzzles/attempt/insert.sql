-- A closed account keeps no attempts; one sent while it closes waits for it.
INSERT INTO puzzle.puzzle_attempts (id, user_id, puzzle_id, solved, duration_ms, rated)
SELECT $1, $2, $3, $4, $5, $6
FROM account.users
WHERE users.id = $2 AND users.deleted_at IS NULL
FOR SHARE
