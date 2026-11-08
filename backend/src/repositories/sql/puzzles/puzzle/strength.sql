SELECT
    puzzles.rating,
    puzzles.rating_deviation
FROM puzzle.puzzles
WHERE puzzles.id = $1
