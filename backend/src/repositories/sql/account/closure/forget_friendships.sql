DELETE FROM social.friendships
WHERE friendships.user_a_id = $1 OR friendships.user_b_id = $1
