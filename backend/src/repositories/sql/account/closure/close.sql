-- The row stays, so the games and battles that name it stay valid for the other players; it
-- can never sign in again, and its email and username are free for anyone.
UPDATE account.users
SET
    username = 'deleted_' || left(replace(users.id::text, '-', ''), 8),
    email = users.id::text || '@deleted.invalid',
    password_hash = '!',
    deleted_at = now(),
    updated_at = now()
WHERE users.id = $1
