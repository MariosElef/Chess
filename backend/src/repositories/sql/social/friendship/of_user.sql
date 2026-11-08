-- Everyone $1 is friends with or has a request with, and how.
SELECT
    users.id AS user_id,
    users.username,
    friendships.status AS "status?",
    friendships.requester_id AS "requester_id?"
FROM social.friendships
INNER JOIN account.users
    ON users.id = CASE
        WHEN friendships.user_a_id = $1 THEN friendships.user_b_id
        ELSE friendships.user_a_id
    END
WHERE
    (friendships.user_a_id = $1 OR friendships.user_b_id = $1)
    AND friendships.status IN ('pending', 'accepted')
ORDER BY lower(users.username)
