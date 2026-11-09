-- Whether $1 has blocked any of the players $2, or any of them has blocked $1.
SELECT EXISTS (
    SELECT 1
    FROM social.blocks
    WHERE
        (blocks.blocker_id = $1 AND blocks.blocked_id = ANY($2))
        OR (blocks.blocked_id = $1 AND blocks.blocker_id = ANY($2))
) AS "blocked!"
