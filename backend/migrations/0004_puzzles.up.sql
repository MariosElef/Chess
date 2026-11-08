-- Rows come from the Lichess puzzle database (CC0) through the backend's import command, never a migration.
-- The id is Lichess's PuzzleId, so a re-import updates rows instead of duplicating them.
CREATE TABLE puzzle.puzzles (
    id text NOT NULL,
    fen text NOT NULL,
    -- Space-separated UCI; the first move is the opponent's, the player answers from there.
    moves text NOT NULL,
    rating integer NOT NULL,
    rating_deviation integer NOT NULL,
    popularity integer NOT NULL,
    nb_plays integer NOT NULL,
    themes text [] NOT NULL,
    game_url text,
    opening_tags text [] NOT NULL DEFAULT '{}',
    CONSTRAINT puzzles_pkey PRIMARY KEY (id)
);

CREATE INDEX puzzles_rating_idx ON puzzle.puzzles (rating);
CREATE INDEX puzzles_themes_idx ON puzzle.puzzles USING gin (themes);

CREATE TABLE puzzle.puzzle_attempts (
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    puzzle_id text NOT NULL,
    solved boolean NOT NULL,
    duration_ms integer NOT NULL,
    rated boolean NOT NULL DEFAULT true,
    attempted_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT puzzle_attempts_pkey PRIMARY KEY (id),
    CONSTRAINT puzzle_attempts_user_id_fkey FOREIGN KEY (user_id) REFERENCES account.users (id) ON DELETE CASCADE,
    CONSTRAINT puzzle_attempts_puzzle_id_fkey FOREIGN KEY (puzzle_id) REFERENCES puzzle.puzzles (id),
    CONSTRAINT puzzle_attempts_duration_ms_check CHECK (duration_ms >= 0)
);

CREATE INDEX puzzle_attempts_user_id_idx ON puzzle.puzzle_attempts (user_id, attempted_at DESC);

ALTER TABLE play.rating_history
ADD CONSTRAINT rating_history_puzzle_attempt_id_fkey FOREIGN KEY (puzzle_attempt_id)
REFERENCES puzzle.puzzle_attempts (id);
