-- Live battles are held in the server's memory: after a restart, the ones left running are over.
UPDATE puzzle.battles
SET finished_at = now()
WHERE battles.finished_at IS NULL
