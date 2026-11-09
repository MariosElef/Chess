-- Up to $3 of the player $1's puzzle attempts before the attempt $2 (all, without it), the
-- latest first, with each puzzle's rating; ids are uuid v7, so they sort by when.
SELECT
    puzzle_attempts.id,
    puzzle_attempts.puzzle_id,
    (SELECT puzzles.rating FROM puzzle.puzzles WHERE puzzles.id = puzzle_attempts.puzzle_id)
        AS puzzle_rating,
    puzzle_attempts.solved,
    puzzle_attempts.duration_ms,
    puzzle_attempts.rated,
    floor(extract(EPOCH FROM now() - puzzle_attempts.attempted_at))::bigint AS "ago_s!"
FROM puzzle.puzzle_attempts
WHERE puzzle_attempts.user_id = $1 AND ($2::uuid IS NULL OR puzzle_attempts.id < $2)
ORDER BY puzzle_attempts.id DESC
LIMIT $3
