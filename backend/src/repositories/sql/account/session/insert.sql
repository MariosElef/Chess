INSERT INTO account.sessions (id, user_id, token_hash, user_agent, expires_at)
VALUES ($1, $2, $3, $4, now() + make_interval(hours => $5))
RETURNING sessions.expires_at
