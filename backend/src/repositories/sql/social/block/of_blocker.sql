-- Everyone $1 has blocked whose account is open, by name.
SELECT
    users.id,
    users.username
FROM social.blocks
INNER JOIN account.users ON users.id = blocks.blocked_id
WHERE blocks.blocker_id = $1 AND users.deleted_at IS NULL
ORDER BY lower(users.username)
