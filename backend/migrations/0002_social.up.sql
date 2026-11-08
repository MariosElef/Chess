-- One row per pair of users, the smaller id first; requester_id and blocked_by_id keep the direction.
-- A block overwrites the row whatever it was, and only blocked_by_id can lift it (by deleting the row).
CREATE TABLE social.friendships (
    user_a_id uuid NOT NULL,
    user_b_id uuid NOT NULL,
    requester_id uuid NOT NULL,
    status text NOT NULL,
    blocked_by_id uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT friendships_pkey PRIMARY KEY (user_a_id, user_b_id),
    CONSTRAINT friendships_user_a_id_fkey FOREIGN KEY (user_a_id) REFERENCES account.users (id) ON DELETE CASCADE,
    CONSTRAINT friendships_user_b_id_fkey FOREIGN KEY (user_b_id) REFERENCES account.users (id) ON DELETE CASCADE,
    CONSTRAINT friendships_order_check CHECK (user_a_id < user_b_id),
    CONSTRAINT friendships_requester_id_check CHECK (requester_id IN (user_a_id, user_b_id)),
    CONSTRAINT friendships_status_check CHECK (status IN ('pending', 'accepted', 'declined', 'blocked')),
    CONSTRAINT friendships_blocked_by_id_check CHECK (
        (status = 'blocked') = (blocked_by_id IS NOT NULL)
        AND (blocked_by_id IS NULL OR blocked_by_id IN (user_a_id, user_b_id))
    )
);

-- The primary key already serves lookups by user_a_id.
CREATE INDEX friendships_user_b_id_idx ON social.friendships (user_b_id);

-- One model for direct messages, group chats and a game's chat.
CREATE TABLE social.conversations (
    id uuid NOT NULL,
    kind text NOT NULL,
    -- Groups only.
    title text,
    -- Direct only: '<smaller user id>:<larger user id>', so a pair has one conversation.
    direct_key text,
    created_by_id uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT conversations_pkey PRIMARY KEY (id),
    CONSTRAINT conversations_created_by_id_fkey FOREIGN KEY (created_by_id) REFERENCES account.users (id),
    CONSTRAINT conversations_kind_check CHECK (kind IN ('direct', 'group', 'game')),
    CONSTRAINT conversations_direct_key_check CHECK ((kind = 'direct') = (direct_key IS NOT NULL)),
    CONSTRAINT conversations_direct_key_key UNIQUE (direct_key)
);

CREATE TABLE social.conversation_members (
    conversation_id uuid NOT NULL,
    user_id uuid NOT NULL,
    role text NOT NULL DEFAULT 'member',
    joined_at timestamptz NOT NULL DEFAULT now(),
    -- Unread messages are the ones sent after this.
    last_read_at timestamptz,
    CONSTRAINT conversation_members_pkey PRIMARY KEY (conversation_id, user_id),
    CONSTRAINT conversation_members_conversation_id_fkey FOREIGN KEY (conversation_id)
    REFERENCES social.conversations (id) ON DELETE CASCADE,
    CONSTRAINT conversation_members_user_id_fkey FOREIGN KEY (user_id) REFERENCES account.users (id),
    CONSTRAINT conversation_members_role_check CHECK (role IN ('owner', 'member'))
);

CREATE INDEX conversation_members_user_id_idx ON social.conversation_members (user_id);

CREATE TABLE social.messages (
    -- uuid v7, so the id also breaks ties between messages sent in the same instant.
    id uuid NOT NULL,
    conversation_id uuid NOT NULL,
    sender_id uuid NOT NULL,
    body text NOT NULL,
    sent_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT messages_pkey PRIMARY KEY (id),
    CONSTRAINT messages_conversation_id_fkey FOREIGN KEY (conversation_id)
    REFERENCES social.conversations (id) ON DELETE CASCADE,
    CONSTRAINT messages_sender_id_fkey FOREIGN KEY (sender_id) REFERENCES account.users (id),
    CONSTRAINT messages_body_check CHECK (length(body) BETWEEN 1 AND 2000)
);

CREATE INDEX messages_conversation_id_sent_at_idx ON social.messages (conversation_id, sent_at, id);
