-- Settles a pending challenge: accepted with its game, or declined, cancelled or expired.
UPDATE play.challenges
SET status = $2, game_id = $3, resolved_at = now()
WHERE challenges.id = $1 AND challenges.status = 'pending'
