UPDATE account.users
SET last_online_at = now()
WHERE users.id = $1
