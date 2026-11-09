-- Whether $2 has been signed in since the game $1 ended, on a session from before then: a
-- shared guest account handed to someone new since has only a newer one.
SELECT EXISTS (
    SELECT 1
    FROM account.sessions
    WHERE
        sessions.user_id = $2
        AND sessions.revoked_at IS NULL
        AND sessions.expires_at > now()
        AND sessions.created_at <= (
            SELECT games.finished_at
            FROM play.games
            WHERE games.id = $1
        )
) AS "seated!"
