-- A closed account keeps no scores; one sent while it closes waits for it.
INSERT INTO puzzle.rush_runs (id, user_id, kind, score)
SELECT $1, $2, $3, $4
FROM account.users
WHERE users.id = $2 AND users.deleted_at IS NULL
FOR SHARE
