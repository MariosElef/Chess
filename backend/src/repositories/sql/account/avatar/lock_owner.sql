-- Holds $1's account until the transaction ends, if it is still open: a picture never lands
-- on a closed account, and two changes to one picture take turns.
SELECT users.id
FROM account.users
WHERE users.id = $1 AND users.deleted_at IS NULL
FOR UPDATE
