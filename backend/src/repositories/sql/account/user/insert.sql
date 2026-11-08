-- Losing a race for the email or username inserts nothing; the service says which was taken.
INSERT INTO account.users (id, email, username, password_hash)
VALUES ($1, $2, $3, $4)
ON CONFLICT DO NOTHING
