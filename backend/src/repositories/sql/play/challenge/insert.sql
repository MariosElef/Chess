INSERT INTO play.challenges (
    id, challenger_id, challenged_id, initial_s, increment_s, rated, challenger_colour,
    expires_at
)
VALUES ($1, $2, $3, $4, $5, $6, 'random', now() + make_interval(secs => $7))
