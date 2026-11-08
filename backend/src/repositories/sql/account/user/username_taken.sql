SELECT EXISTS (
    SELECT 1
    FROM account.users
    WHERE lower(users.username) = lower($1) AND users.deleted_at IS NULL
) AS "taken!"
