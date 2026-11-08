-- A random puzzle in a rating range is the first one at or after a random (rating, id) point:
-- one index walk instead of counting the range.
CREATE INDEX puzzles_rating_id_idx ON puzzle.puzzles (rating, id);
DROP INDEX puzzle.puzzles_rating_idx;
