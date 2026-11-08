UPDATE puzzle.battles
SET
    first_score = $2,
    second_score = $3,
    winner_id = $4,
    finished_at = now()
WHERE battles.id = $1
