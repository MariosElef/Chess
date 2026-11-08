-- When the player last had the app open: set as they connect and as their last socket goes,
-- for friends' "Last online".
ALTER TABLE account.users ADD COLUMN last_online_at timestamptz;
