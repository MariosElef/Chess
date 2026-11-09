-- A made-up account, joined at $4; a taken username inserts nothing.
INSERT INTO account.users (id, username, password_hash, created_at, updated_at)
VALUES ($1, $2, $3, $4, $4)
ON CONFLICT DO NOTHING
