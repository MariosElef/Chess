-- A request from $3, new or asked again after it was declined or withdrawn.
INSERT INTO social.friendships (user_a_id, user_b_id, requester_id, status)
VALUES ($1, $2, $3, 'pending')
ON CONFLICT (user_a_id, user_b_id) DO UPDATE
SET requester_id = excluded.requester_id, status = 'pending', updated_at = now()
WHERE friendships.status <> 'blocked'
