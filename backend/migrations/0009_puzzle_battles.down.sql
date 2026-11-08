DELETE FROM play.rating_history WHERE battle_id IS NOT NULL;
ALTER TABLE play.rating_history
DROP CONSTRAINT rating_history_source_check,
DROP COLUMN battle_id,
ADD CONSTRAINT rating_history_source_check CHECK (num_nonnulls(game_id, puzzle_attempt_id) = 1);
DROP TABLE puzzle.battles;
DELETE FROM play.ratings WHERE category_id = 'puzzle_battle';
DELETE FROM play.rating_categories WHERE id = 'puzzle_battle';
