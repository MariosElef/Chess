UPDATE account.sessions
SET revoked_at = now()
WHERE sessions.id = $1 AND sessions.revoked_at IS NULL
