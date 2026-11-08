-- Everyone with a request waiting to or from the player: their requests change once it closes.
SELECT
    CASE
        WHEN friendships.user_a_id = $1 THEN friendships.user_b_id
        ELSE friendships.user_a_id
    END AS "id!"
FROM social.friendships
WHERE
    (friendships.user_a_id = $1 OR friendships.user_b_id = $1)
    AND friendships.status = 'pending'
