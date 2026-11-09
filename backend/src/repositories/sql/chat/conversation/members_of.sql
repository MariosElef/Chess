-- The members of each of the conversations $1, by name, and which of them owns it.
SELECT
    conversation_members.conversation_id,
    users.id AS user_id,
    users.username,
    conversation_members.role = 'owner' AS "owner!"
FROM social.conversation_members
INNER JOIN account.users ON users.id = conversation_members.user_id
WHERE conversation_members.conversation_id = ANY($1)
ORDER BY lower(users.username)
