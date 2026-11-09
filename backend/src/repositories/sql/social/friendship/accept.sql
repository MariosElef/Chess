-- Accepts the pending request $3 made; nothing if there is none or either blocked the other.
UPDATE social.friendships
SET status = 'accepted', updated_at = now()
WHERE
    friendships.user_a_id = $1
    AND friendships.user_b_id = $2
    AND friendships.status = 'pending'
    AND friendships.requester_id = $3
    AND NOT EXISTS (
        SELECT 1
        FROM social.blocks
        WHERE
            (blocks.blocker_id = $1 AND blocks.blocked_id = $2)
            OR (blocks.blocker_id = $2 AND blocks.blocked_id = $1)
    )
