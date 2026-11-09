-- Everyone $1 is friends with or has a request with, how, and how many seconds ago they last
-- had the app open; before that was first stored, when their session was last used.
SELECT
    users.id AS user_id,
    users.username,
    users.avatar_hash AS avatar,
    floor(extract(EPOCH FROM now() - coalesce(
        users.last_online_at,
        (
            SELECT max(sessions.last_seen_at)
            FROM account.sessions
            WHERE sessions.user_id = users.id
        )
    )))::bigint AS away_s,
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
