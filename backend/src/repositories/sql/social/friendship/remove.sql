-- Unfriends, declines or withdraws: the pair goes back to strangers. A block stays.
DELETE FROM social.friendships
WHERE
    friendships.user_a_id = $1
    AND friendships.user_b_id = $2
    AND friendships.status <> 'blocked'
