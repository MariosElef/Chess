-- Up to $3 of the player $1's finished battles before the battle $2 (all, without it), the
-- latest first, with the other player's name; ids are uuid v7, so they sort by start. Each seat
-- reads its own index.
SELECT
    battles.id,
    battles.first_id,
    (
        SELECT users.username
        FROM account.users
        WHERE users.id = CASE
            WHEN battles.first_id = $1 THEN battles.second_id
            ELSE battles.first_id
        END
    ) AS "opponent!",
    battles.first_score,
    battles.second_score,
    battles.winner_id,
    battles.rated,
    floor(extract(EPOCH FROM now() - battles.finished_at))::bigint AS "ago_s!"
FROM puzzle.battles
WHERE battles.id IN (
    (
        SELECT battles.id
        FROM puzzle.battles
        WHERE
            battles.first_id = $1
            AND battles.finished_at IS NOT NULL
            AND ($2::uuid IS NULL OR battles.id < $2)
        ORDER BY battles.id DESC
        LIMIT $3
    )
    UNION ALL
    (
        SELECT battles.id
        FROM puzzle.battles
        WHERE
            battles.second_id = $1
            AND battles.finished_at IS NOT NULL
            AND ($2::uuid IS NULL OR battles.id < $2)
        ORDER BY battles.id DESC
        LIMIT $3
    )
)
ORDER BY battles.id DESC
LIMIT $3
