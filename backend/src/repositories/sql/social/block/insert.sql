-- $1 blocks $2; blocking again changes nothing.
INSERT INTO social.blocks (blocker_id, blocked_id)
VALUES ($1, $2)
ON CONFLICT (blocker_id, blocked_id) DO NOTHING
