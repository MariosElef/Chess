-- Accepts the pending request $3 made; nothing if there is none.
UPDATE social.friendships
SET status = 'accepted', updated_at = now()
WHERE
    friendships.user_a_id = $1
    AND friendships.user_b_id = $2
    AND friendships.status = 'pending'
    AND friendships.requester_id = $3
