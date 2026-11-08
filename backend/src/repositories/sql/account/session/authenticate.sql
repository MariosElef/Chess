UPDATE account.sessions
SET last_seen_at = now()
FROM account.users
WHERE
    sessions.token_hash = $1
    AND sessions.revoked_at IS NULL
    AND sessions.expires_at > now()
    AND users.id = sessions.user_id
    AND users.deleted_at IS NULL
RETURNING
    sessions.id AS session_id,
    sessions.user_id
