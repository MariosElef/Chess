-- Held until the account is closed: anything rating or recording the player waits, then finds it
-- closed.
SELECT users.id
FROM account.users
WHERE users.id = $1
FOR UPDATE
