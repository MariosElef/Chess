-- The members of each of the conversations $1, by name.
SELECT
    conversation_members.conversation_id,
    users.id AS user_id,
    users.username
FROM social.conversation_members
INNER JOIN account.users ON users.id = conversation_members.user_id
WHERE conversation_members.conversation_id = ANY($1)
ORDER BY lower(users.username)
