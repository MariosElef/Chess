-- Every picture goes, and players show their initials again.
DROP TABLE account.avatars;
ALTER TABLE account.users DROP COLUMN avatar_hash;
