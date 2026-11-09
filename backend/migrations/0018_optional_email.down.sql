-- Accounts without an email get a placeholder no one can receive mail at, so the column can be
-- required again.
UPDATE account.users SET email = users.id::text || '@no-email.invalid' WHERE email IS NULL;
ALTER TABLE account.users ALTER COLUMN email SET NOT NULL;
