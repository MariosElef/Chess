SELECT EXISTS (
    SELECT 1
    FROM account.users
    WHERE users.email = $1 AND users.deleted_at IS NULL
) AS "taken!"
