ALTER TABLE play.rating_history DROP CONSTRAINT rating_history_puzzle_attempt_id_fkey;

DROP TABLE puzzle.puzzle_attempts;
DROP TABLE puzzle.puzzles;
