-- What closing an account checks: its password, and that it is not a shared guest.
SELECT
    users.password_hash,
    users.guest_number IS NOT NULL AS "guest!"
FROM account.users
WHERE users.id = $1 AND users.deleted_at IS NULL
