-- Back to a block in the pair's friendship row; where both blocked, the earlier block is kept.
ALTER TABLE social.friendships DROP CONSTRAINT friendships_status_check;
ALTER TABLE social.friendships ADD CONSTRAINT friendships_status_check
CHECK (status IN ('pending', 'accepted', 'declined', 'blocked'));
ALTER TABLE social.friendships ADD COLUMN blocked_by_id uuid;

INSERT INTO social.friendships (user_a_id, user_b_id, requester_id, status, blocked_by_id)
SELECT DISTINCT ON (least(blocks.blocker_id, blocks.blocked_id), greatest(blocks.blocker_id, blocks.blocked_id))
    least(blocks.blocker_id, blocks.blocked_id),
    greatest(blocks.blocker_id, blocks.blocked_id),
    blocks.blocker_id,
    'blocked',
    blocks.blocker_id
FROM social.blocks
ORDER BY
    least(blocks.blocker_id, blocks.blocked_id),
    greatest(blocks.blocker_id, blocks.blocked_id),
    blocks.created_at
ON CONFLICT (user_a_id, user_b_id) DO UPDATE
SET requester_id = excluded.requester_id, status = 'blocked', blocked_by_id = excluded.blocked_by_id;

ALTER TABLE social.friendships ADD CONSTRAINT friendships_blocked_by_id_check CHECK (
    (status = 'blocked') = (blocked_by_id IS NOT NULL)
    AND (blocked_by_id IS NULL OR blocked_by_id IN (user_a_id, user_b_id))
);
DROP TABLE social.blocks;
