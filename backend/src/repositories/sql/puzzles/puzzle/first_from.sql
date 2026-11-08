-- Walks puzzles_rating_id_idx from the given point, so a random point gives a random puzzle,
-- skipping the ones the player has already attempted.
SELECT
    puzzles.id,
    puzzles.fen,
    puzzles.moves,
    puzzles.rating,
    puzzles.nb_plays,
    puzzles.themes
FROM puzzle.puzzles
WHERE
    puzzles.rating BETWEEN $1 AND $2
    AND (puzzles.rating, puzzles.id) >= ($3, $4)
    AND NOT EXISTS (
        SELECT 1
        FROM puzzle.puzzle_attempts
        WHERE puzzle_attempts.user_id = $5 AND puzzle_attempts.puzzle_id = puzzles.id
    )
ORDER BY puzzles.rating, puzzles.id
LIMIT 1
