-- $1 lifts their block on $2; a block $2 made on $1 stays.
DELETE FROM social.blocks
WHERE blocks.blocker_id = $1 AND blocks.blocked_id = $2
