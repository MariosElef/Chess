-- Bullet, blitz, rapid, classical and puzzle; the rows are seeded in 0005.
-- A game's category comes from its estimated length: initial_s + 40 * increment_s.
CREATE TABLE play.rating_categories (
    id text NOT NULL,
    min_estimate_s integer,
    max_estimate_s integer,
    sort_order integer NOT NULL,
    CONSTRAINT rating_categories_pkey PRIMARY KEY (id)
);

-- game_moves is the source of truth; ply_count, fen and the clocks are its cached result,
-- rewritten in the same transaction as each move. The side to move is ply_count % 2.
-- The mover's time left is their clock minus (now() - last_move_at); a server tick over
-- games_ongoing_idx ends games whose clock ran out with outcome 'timeout'.
-- Spectators need no table: public games are broadcast in process.
CREATE TABLE play.games (
    id uuid NOT NULL,
    white_id uuid NOT NULL,
    black_id uuid NOT NULL,
    category_id text NOT NULL,
    initial_s integer NOT NULL,
    increment_s integer NOT NULL,
    rated boolean NOT NULL,
    public boolean NOT NULL DEFAULT true,
    conversation_id uuid,
    status text NOT NULL DEFAULT 'ongoing',
    outcome text,
    -- NULL for a draw.
    winner text,
    ply_count integer NOT NULL DEFAULT 0,
    fen text NOT NULL,
    white_clock_ms integer NOT NULL,
    black_clock_ms integer NOT NULL,
    -- NULL until the first move; before it, the abandonment rule applies instead of the clock.
    last_move_at timestamptz,
    -- One open offer at a time, cleared by any move or a decline.
    draw_offer_by text,
    started_at timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT games_pkey PRIMARY KEY (id),
    CONSTRAINT games_white_id_fkey FOREIGN KEY (white_id) REFERENCES account.users (id),
    CONSTRAINT games_black_id_fkey FOREIGN KEY (black_id) REFERENCES account.users (id),
    CONSTRAINT games_category_id_fkey FOREIGN KEY (category_id) REFERENCES play.rating_categories (id),
    CONSTRAINT games_conversation_id_fkey FOREIGN KEY (conversation_id) REFERENCES social.conversations (id),
    CONSTRAINT games_players_check CHECK (white_id <> black_id),
    CONSTRAINT games_time_control_check CHECK (initial_s > 0 AND increment_s >= 0),
    CONSTRAINT games_status_check CHECK (status IN ('ongoing', 'finished', 'aborted')),
    CONSTRAINT games_outcome_check CHECK (
        outcome IN (
            'checkmate', 'stalemate', 'resignation', 'draw_agreement', 'timeout',
            'abandonment', 'insufficient_material', 'repetition', 'fifty_move'
        )
    ),
    CONSTRAINT games_winner_check CHECK (
        winner IS NULL
        OR (winner IN ('white', 'black') AND outcome IN ('checkmate', 'resignation', 'timeout', 'abandonment'))
    ),
    CONSTRAINT games_draw_offer_by_check CHECK (draw_offer_by IN ('white', 'black')),
    CONSTRAINT games_finished_check CHECK (
        (status = 'finished') = (outcome IS NOT NULL)
        AND (status = 'ongoing') = (finished_at IS NULL)
    ),
    CONSTRAINT games_ply_count_check CHECK (ply_count >= 0)
);

CREATE INDEX games_white_id_idx ON play.games (white_id, started_at DESC);
CREATE INDEX games_black_id_idx ON play.games (black_id, started_at DESC);
CREATE INDEX games_ongoing_idx ON play.games (last_move_at) WHERE status = 'ongoing';

-- One row per half-move; the primary key rejects a move submitted twice.
CREATE TABLE play.game_moves (
    game_id uuid NOT NULL,
    -- 1-based; odd plies are White's.
    ply integer NOT NULL,
    uci text NOT NULL,
    -- Both clocks after this move, increment included.
    white_clock_ms integer NOT NULL,
    black_clock_ms integer NOT NULL,
    played_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT game_moves_pkey PRIMARY KEY (game_id, ply),
    CONSTRAINT game_moves_game_id_fkey FOREIGN KEY (game_id) REFERENCES play.games (id) ON DELETE CASCADE,
    CONSTRAINT game_moves_ply_check CHECK (ply >= 1),
    CONSTRAINT game_moves_uci_check CHECK (uci ~ '^[a-h][1-8][a-h][1-8][qrbn]?$')
);

-- A challenge to a friend, or an open seek when challenged_id is NULL; accepting creates the game.
CREATE TABLE play.challenges (
    id uuid NOT NULL,
    challenger_id uuid NOT NULL,
    challenged_id uuid,
    initial_s integer NOT NULL,
    increment_s integer NOT NULL,
    rated boolean NOT NULL,
    challenger_colour text NOT NULL,
    status text NOT NULL DEFAULT 'pending',
    game_id uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    resolved_at timestamptz,
    CONSTRAINT challenges_pkey PRIMARY KEY (id),
    CONSTRAINT challenges_challenger_id_fkey FOREIGN KEY (challenger_id) REFERENCES account.users (id),
    CONSTRAINT challenges_challenged_id_fkey FOREIGN KEY (challenged_id) REFERENCES account.users (id),
    CONSTRAINT challenges_game_id_fkey FOREIGN KEY (game_id) REFERENCES play.games (id),
    CONSTRAINT challenges_players_check CHECK (challenger_id <> challenged_id),
    CONSTRAINT challenges_time_control_check CHECK (initial_s > 0 AND increment_s >= 0),
    CONSTRAINT challenges_challenger_colour_check CHECK (challenger_colour IN ('white', 'black', 'random')),
    CONSTRAINT challenges_status_check CHECK (
        status IN ('pending', 'accepted', 'declined', 'cancelled', 'expired')
    ),
    CONSTRAINT challenges_game_id_check CHECK ((status = 'accepted') = (game_id IS NOT NULL)),
    CONSTRAINT challenges_resolved_at_check CHECK ((status = 'pending') = (resolved_at IS NULL))
);

CREATE INDEX challenges_challenged_id_idx ON play.challenges (challenged_id) WHERE status = 'pending';
CREATE INDEX challenges_open_idx ON play.challenges (created_at)
WHERE status = 'pending' AND challenged_id IS NULL;

-- The current Glicko-2 rating per user and category.
CREATE TABLE play.ratings (
    user_id uuid NOT NULL,
    category_id text NOT NULL,
    rating double precision NOT NULL DEFAULT 1500,
    deviation double precision NOT NULL DEFAULT 350,
    volatility double precision NOT NULL DEFAULT 0.06,
    games_played integer NOT NULL DEFAULT 0,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ratings_pkey PRIMARY KEY (user_id, category_id),
    CONSTRAINT ratings_user_id_fkey FOREIGN KEY (user_id) REFERENCES account.users (id) ON DELETE CASCADE,
    CONSTRAINT ratings_category_id_fkey FOREIGN KEY (category_id) REFERENCES play.rating_categories (id)
);

-- One row per rated game or puzzle attempt, for rating graphs; the puzzle foreign key is added in 0004.
CREATE TABLE play.rating_history (
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    category_id text NOT NULL,
    game_id uuid,
    puzzle_attempt_id uuid,
    rating double precision NOT NULL,
    deviation double precision NOT NULL,
    volatility double precision NOT NULL,
    recorded_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT rating_history_pkey PRIMARY KEY (id),
    CONSTRAINT rating_history_user_id_fkey FOREIGN KEY (user_id) REFERENCES account.users (id) ON DELETE CASCADE,
    CONSTRAINT rating_history_category_id_fkey FOREIGN KEY (category_id) REFERENCES play.rating_categories (id),
    CONSTRAINT rating_history_game_id_fkey FOREIGN KEY (game_id) REFERENCES play.games (id),
    CONSTRAINT rating_history_source_check CHECK (num_nonnulls(game_id, puzzle_attempt_id) = 1)
);

CREATE INDEX rating_history_user_id_category_id_idx ON play.rating_history (user_id, category_id, recorded_at);
