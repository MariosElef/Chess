-- Whether the session $1 is still signed in: not revoked and not expired.
SELECT EXISTS (
    SELECT 1
    FROM account.sessions
    WHERE sessions.id = $1 AND sessions.revoked_at IS NULL AND sessions.expires_at > now()
) AS "live!"
