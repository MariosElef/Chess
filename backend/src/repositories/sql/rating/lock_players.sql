-- The players' rows, shared and in id order, before their ratings: closing an account takes
-- its row before its ratings too, so the two never wait on each other.
SELECT users.id
FROM account.users
WHERE users.id = ANY($1)
ORDER BY users.id
FOR SHARE
