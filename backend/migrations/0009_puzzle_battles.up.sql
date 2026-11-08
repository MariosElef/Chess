-- Puzzle Battle: two players race through the same puzzles for three minutes. Its rating is a
-- category of its own; a battle is stored when it starts and finished when it ends.
INSERT INTO play.rating_categories (id, min_estimate_s, max_estimate_s, sort_order) VALUES
('puzzle_battle', NULL, NULL, 6);

CREATE TABLE puzzle.battles (
    id uuid NOT NULL,
    first_id uuid NOT NULL,
    second_id uuid NOT NULL,
    rated boolean NOT NULL,
    first_score integer NOT NULL DEFAULT 0,
    second_score integer NOT NULL DEFAULT 0,
    -- NULL for a draw, and while it is being played.
    winner_id uuid,
    started_at timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz,
    CONSTRAINT battles_pkey PRIMARY KEY (id),
    CONSTRAINT battles_first_id_fkey FOREIGN KEY (first_id) REFERENCES account.users (id) ON DELETE CASCADE,
    CONSTRAINT battles_second_id_fkey FOREIGN KEY (second_id) REFERENCES account.users (id) ON DELETE CASCADE,
    CONSTRAINT battles_winner_check CHECK (winner_id IS NULL OR winner_id IN (first_id, second_id)),
    CONSTRAINT battles_scores_check CHECK (first_score >= 0 AND second_score >= 0)
);

-- A player's wins this month, for the Puzzle Battle panel.
CREATE INDEX battles_winner_id_finished_at_idx ON puzzle.battles (winner_id, finished_at);

-- A battle moves a rating too: the history names it instead of a game or a puzzle attempt.
ALTER TABLE play.rating_history
ADD COLUMN battle_id uuid,
ADD CONSTRAINT rating_history_battle_id_fkey FOREIGN KEY (battle_id) REFERENCES puzzle.battles (id),
DROP CONSTRAINT rating_history_source_check,
ADD CONSTRAINT rating_history_source_check CHECK (
    num_nonnulls(game_id, puzzle_attempt_id, battle_id) = 1
);
