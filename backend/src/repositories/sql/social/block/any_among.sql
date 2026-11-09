-- Whether anyone among the players $1 has blocked another of them.
SELECT EXISTS (
    SELECT 1
    FROM social.blocks
    WHERE blocks.blocker_id = ANY($1) AND blocks.blocked_id = ANY($1)
) AS "blocked!"
