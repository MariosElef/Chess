-- Profile pictures: one square JPEG per account, named by the hash of its bytes, which the
-- account row carries so every list of players can say which picture to show.
ALTER TABLE account.users ADD COLUMN avatar_hash text;

CREATE TABLE account.avatars (
    user_id uuid NOT NULL,
    hash text NOT NULL,
    image bytea NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT avatars_pkey PRIMARY KEY (user_id),
    CONSTRAINT avatars_user_id_fkey FOREIGN KEY (user_id) REFERENCES account.users (id) ON DELETE CASCADE
);
