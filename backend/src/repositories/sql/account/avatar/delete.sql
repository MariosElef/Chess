-- $1 has no picture any more.
DELETE FROM account.avatars
WHERE avatars.user_id = $1
