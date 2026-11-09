-- Blocks get a table of their own, so each of a pair can block the other and lift only their
-- own; a friendship row no longer holds one.
CREATE TABLE social.blocks (
    blocker_id uuid NOT NULL,
    blocked_id uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT blocks_pkey PRIMARY KEY (blocker_id, blocked_id),
    CONSTRAINT blocks_blocker_id_fkey FOREIGN KEY (blocker_id) REFERENCES account.users (id) ON DELETE CASCADE,
    CONSTRAINT blocks_blocked_id_fkey FOREIGN KEY (blocked_id) REFERENCES account.users (id) ON DELETE CASCADE,
    CONSTRAINT blocks_not_self_check CHECK (blocker_id <> blocked_id)
);

CREATE INDEX blocks_blocked_id_idx ON social.blocks (blocked_id);

INSERT INTO social.blocks (blocker_id, blocked_id, created_at)
SELECT
    friendships.blocked_by_id,
    CASE
        WHEN friendships.blocked_by_id = friendships.user_a_id THEN friendships.user_b_id
        ELSE friendships.user_a_id
    END,
    friendships.updated_at
FROM social.friendships
WHERE friendships.status = 'blocked';

DELETE FROM social.friendships WHERE friendships.status = 'blocked';

ALTER TABLE social.friendships DROP CONSTRAINT friendships_blocked_by_id_check;
ALTER TABLE social.friendships DROP COLUMN blocked_by_id;
ALTER TABLE social.friendships DROP CONSTRAINT friendships_status_check;
ALTER TABLE social.friendships ADD CONSTRAINT friendships_status_check
CHECK (status IN ('pending', 'accepted', 'declined'));
