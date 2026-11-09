-- Which of $1 and $2 has blocked the other: none, one or both.
SELECT blocks.blocker_id
FROM social.blocks
WHERE
    (blocks.blocker_id = $1 AND blocks.blocked_id = $2)
    OR (blocks.blocker_id = $2 AND blocks.blocked_id = $1)
