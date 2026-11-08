-- Up to $3 of the conversation $1's messages before the message $2 (all, without it), the
-- latest first; ids are uuid v7, so they sort by when they were sent.
SELECT
    messages.id,
    messages.conversation_id,
    messages.sender_id,
    users.username AS sender_name,
    messages.body,
    messages.sent_at
FROM social.messages
INNER JOIN account.users ON users.id = messages.sender_id
WHERE messages.conversation_id = $1 AND ($2::uuid IS NULL OR messages.id < $2)
ORDER BY messages.id DESC
LIMIT $3
