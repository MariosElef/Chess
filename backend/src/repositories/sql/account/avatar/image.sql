-- $1's picture, if $2 is still the hash their account hands out and the account is open.
SELECT avatars.image
FROM account.avatars
INNER JOIN account.users ON users.id = avatars.user_id
WHERE
    avatars.user_id = $1
    AND avatars.hash = $2
    AND users.avatar_hash = $2
    AND users.deleted_at IS NULL
