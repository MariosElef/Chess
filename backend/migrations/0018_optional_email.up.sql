-- An account needs only a username and a password; the email is optional. The unique index on
-- open accounts' emails stays: Postgres lets any number of accounts have none.
ALTER TABLE account.users ALTER COLUMN email DROP NOT NULL;
