UPDATE play.games
SET draw_offer_by = $2, updated_at = now()
WHERE games.id = $1
