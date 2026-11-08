-- Players whose name contains $2 (lowercase, LIKE-escaped), names starting with it first;
-- never $1. With the friendship to $1, if any.
SELECT
    users.id AS user_id,
    users.username,
    friendships.status AS "status?",
    friendships.requester_id AS "requester_id?"
FROM account.users
LEFT JOIN social.friendships
    ON
        friendships.user_a_id = least($1, users.id)
        AND friendships.user_b_id = greatest($1, users.id)
WHERE
    users.id <> $1
    AND lower(users.username) LIKE '%' || $2 || '%' ESCAPE '\'
ORDER BY lower(users.username) LIKE $2 || '%' DESC, lower(users.username)
LIMIT $3
