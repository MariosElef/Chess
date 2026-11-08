-- The first guest with no live session, locked until the transaction ends so a sign-in running
-- at the same moment skips it and takes the next one.
SELECT
    users.id,
    users.email,
    users.username,
    users.created_at,
    users.guest_number IS NOT NULL AS "guest!"
FROM account.users
WHERE
    users.guest_number IS NOT NULL
    AND NOT EXISTS (
        SELECT 1
        FROM account.sessions
        WHERE
            sessions.user_id = users.id
            AND sessions.revoked_at IS NULL
            AND sessions.expires_at > now()
    )
ORDER BY users.guest_number
LIMIT 1
FOR UPDATE SKIP LOCKED
