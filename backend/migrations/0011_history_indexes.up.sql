-- Histories page newest first by id (uuid v7, so in time order): one index per way they read.
CREATE INDEX battles_first_id_id_idx ON puzzle.battles (first_id, id DESC);
CREATE INDEX battles_second_id_id_idx ON puzzle.battles (second_id, id DESC);
CREATE INDEX games_white_id_id_idx ON play.games (white_id, id DESC);
CREATE INDEX games_black_id_id_idx ON play.games (black_id, id DESC);
CREATE INDEX puzzle_attempts_user_id_id_idx ON puzzle.puzzle_attempts (user_id, id DESC);
-- A rating change looks up the rating just before, in the same category.
CREATE INDEX rating_history_user_id_category_id_id_idx ON play.rating_history (user_id, category_id, id);
-- Replaced by the ones above: nothing reads in the time columns' order any more.
DROP INDEX play.games_white_id_idx;
DROP INDEX play.games_black_id_idx;
DROP INDEX puzzle.puzzle_attempts_user_id_idx;
DROP INDEX play.rating_history_user_id_category_id_idx;
