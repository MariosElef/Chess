-- account: passwords and sessions; social: friends and chat; play: games and ratings; puzzle: imported puzzles.
CREATE SCHEMA account;
CREATE SCHEMA social;
CREATE SCHEMA play;
CREATE SCHEMA puzzle;

CREATE TABLE account.users (
    id uuid NOT NULL,
    -- Lowercased by the service before it is stored.
    email text NOT NULL,
    username text NOT NULL,
    -- An argon2id PHC string.
    password_hash text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    -- A closed account keeps its games, ratings and messages valid.
    deleted_at timestamptz,
    CONSTRAINT users_pkey PRIMARY KEY (id),
    CONSTRAINT users_username_check CHECK (username ~ '^[A-Za-z0-9_]{3,20}$')
);

CREATE UNIQUE INDEX users_email_key ON account.users (email) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX users_username_key ON account.users (lower(username)) WHERE deleted_at IS NULL;

CREATE TABLE account.sessions (
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    -- sha256 of a random 32-byte token; the raw token is only ever sent to the client.
    token_hash bytea NOT NULL,
    user_agent text,
    created_at timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    CONSTRAINT sessions_pkey PRIMARY KEY (id),
    CONSTRAINT sessions_user_id_fkey FOREIGN KEY (user_id) REFERENCES account.users (id) ON DELETE CASCADE,
    CONSTRAINT sessions_token_hash_key UNIQUE (token_hash)
);

CREATE INDEX sessions_user_id_idx ON account.sessions (user_id);
