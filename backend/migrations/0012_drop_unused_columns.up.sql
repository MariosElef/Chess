-- Columns nothing reads: a game's speed comes from chess-domain's own boundaries, and every
-- game is open to its two players only.
ALTER TABLE play.rating_categories DROP COLUMN min_estimate_s, DROP COLUMN max_estimate_s;
ALTER TABLE play.games DROP COLUMN public;
