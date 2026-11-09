-- The hash of $1's picture, which every list of players hands out; none without one.
UPDATE account.users
SET avatar_hash = $2, updated_at = now()
WHERE users.id = $1
