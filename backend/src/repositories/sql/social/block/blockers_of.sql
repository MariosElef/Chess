-- Everyone who has blocked $1: they still see them in their Blocked list.
SELECT blocks.blocker_id
FROM social.blocks
WHERE blocks.blocked_id = $1
