-- The last message of each of the conversations $1.
SELECT DISTINCT ON (messages.conversation_id)
    messages.id,
    messages.conversation_id,
    messages.sender_id,
    users.username AS sender_name,
    messages.body,
    messages.sent_at
FROM social.messages
INNER JOIN account.users ON users.id = messages.sender_id
WHERE messages.conversation_id = ANY($1)
ORDER BY messages.conversation_id, messages.id DESC
