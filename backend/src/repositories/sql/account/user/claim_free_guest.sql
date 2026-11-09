-- The first guest nobody is using: not among the players the live server has ($1: connected
-- or at a board), and not signed in within the last minute, which covers an app that has
-- signed in but not yet connected. Locked until the transaction ends, so a sign-in running at
-- the same moment skips it and takes the next one.
SELECT
    users.id,
    users.email,
    users.username,
    users.created_at,
    users.guest_number IS NOT NULL AS "guest!",
    users.avatar_hash AS avatar
FROM account.users
WHERE
    users.guest_number IS NOT NULL
    AND NOT (users.id = ANY($1::uuid[]))
    AND NOT EXISTS (
        SELECT 1
        FROM account.sessions
        WHERE
            sessions.user_id = users.id
            AND sessions.revoked_at IS NULL
            AND sessions.expires_at > now()
            AND sessions.created_at > now() - interval '1 minute'
    )
ORDER BY users.guest_number
LIMIT 1
FOR UPDATE SKIP LOCKED
