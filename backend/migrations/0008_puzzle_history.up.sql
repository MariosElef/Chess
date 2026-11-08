-- Whether a player has already attempted a puzzle, checked for every puzzle served.
CREATE INDEX puzzle_attempts_user_id_puzzle_id_idx ON puzzle.puzzle_attempts (user_id, puzzle_id);

-- One row per finished Puzzle Rush; a player's best is the highest score of its kind.
CREATE TABLE puzzle.rush_runs (
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    kind text NOT NULL,
    score integer NOT NULL,
    finished_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT rush_runs_pkey PRIMARY KEY (id),
    CONSTRAINT rush_runs_user_id_fkey FOREIGN KEY (user_id) REFERENCES account.users (id) ON DELETE CASCADE,
    CONSTRAINT rush_runs_kind_check CHECK (kind IN ('three_minutes', 'five_minutes', 'survival')),
    CONSTRAINT rush_runs_score_check CHECK (score >= 0)
);

CREATE INDEX rush_runs_user_id_kind_idx ON puzzle.rush_runs (user_id, kind, score DESC);
