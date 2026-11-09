-- Unfriends, declines or withdraws: the pair goes back to strangers.
DELETE FROM social.friendships
WHERE
    friendships.user_a_id = $1
    AND friendships.user_b_id = $2
