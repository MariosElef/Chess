-- A request from $3, new or asked again after it was declined or withdrawn; none to a closed
-- account or between a pair where either blocked the other, and one sent while it closes waits.
INSERT INTO social.friendships (user_a_id, user_b_id, requester_id, status)
SELECT $1::uuid, $2::uuid, $3::uuid, 'pending'
FROM account.users
WHERE
    users.id = CASE WHEN $3::uuid = $1::uuid THEN $2::uuid ELSE $1::uuid END
    AND users.deleted_at IS NULL
    AND NOT EXISTS (
        SELECT 1
        FROM social.blocks
        WHERE
            (blocks.blocker_id = $1 AND blocks.blocked_id = $2)
            OR (blocks.blocker_id = $2 AND blocks.blocked_id = $1)
    )
FOR SHARE
ON CONFLICT (user_a_id, user_b_id) DO UPDATE
SET requester_id = excluded.requester_id, status = 'pending', updated_at = now()
WHERE friendships.status = 'declined'
