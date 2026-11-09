-- $1's live sessions, the most recently used first, with how long ago each began and was used.
SELECT
    sessions.id,
    sessions.user_agent,
    floor(extract(EPOCH FROM now() - sessions.created_at))::bigint AS "signed_in_s!",
    floor(extract(EPOCH FROM now() - sessions.last_seen_at))::bigint AS "seen_s!"
FROM account.sessions
WHERE
    sessions.user_id = $1
    AND sessions.revoked_at IS NULL
    AND sessions.expires_at > now()
ORDER BY sessions.last_seen_at DESC
