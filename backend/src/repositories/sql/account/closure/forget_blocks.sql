-- Blocks they made, and blocks made on them.
DELETE FROM social.blocks
WHERE blocks.blocker_id = $1 OR blocks.blocked_id = $1
