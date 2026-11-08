-- The friendship of one pair, given smaller id first.
SELECT
    friendships.status,
    friendships.requester_id
FROM social.friendships
WHERE friendships.user_a_id = $1 AND friendships.user_b_id = $2
