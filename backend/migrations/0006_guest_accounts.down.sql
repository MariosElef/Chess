DELETE FROM account.sessions
USING account.users
WHERE sessions.user_id = users.id AND users.guest_number IS NOT NULL;
DELETE FROM account.users WHERE users.guest_number IS NOT NULL;
ALTER TABLE account.users DROP COLUMN guest_number;
