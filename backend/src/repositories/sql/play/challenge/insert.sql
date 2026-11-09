-- A challenge as it is sent: who asks whom, its terms, the challenger's colour ('random' for a
-- friend's challenge), the game a rematch replays, and when it lapses.
INSERT INTO play.challenges (
    id, challenger_id, challenged_id, initial_s, increment_s, rated, challenger_colour,
    rematch_of, expires_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now() + make_interval(secs => $9))
