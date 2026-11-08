-- Live games are held in the server's memory: after a restart, the ones left running are over.
UPDATE play.games
SET status = 'aborted', draw_offer_by = NULL, finished_at = now(), updated_at = now()
WHERE games.status = 'ongoing'
