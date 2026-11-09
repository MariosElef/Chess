SELECT
    users.id,
    users.email,
    users.username,
    users.created_at,
    users.guest_number IS NOT NULL AS "guest!",
    users.avatar_hash AS avatar
FROM account.users
WHERE users.id = $1 AND users.deleted_at IS NULL
