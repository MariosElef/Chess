-- Ends every live session of $1 but $2, the one asking.
UPDATE account.sessions
SET revoked_at = now()
WHERE sessions.user_id = $1 AND sessions.id <> $2 AND sessions.revoked_at IS NULL
