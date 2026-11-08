-- Emails are stored lowercased; usernames keep their case but are unique without it. Guests
-- have no password, so they are never found here.
SELECT
    users.id,
    users.email,
    users.username,
    users.created_at,
    users.guest_number IS NOT NULL AS "guest!",
    users.password_hash
FROM account.users
WHERE
    (users.email = lower($1) OR lower(users.username) = lower($1))
    AND users.deleted_at IS NULL
    AND users.guest_number IS NULL
