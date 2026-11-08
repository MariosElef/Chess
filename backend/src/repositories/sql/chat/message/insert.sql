INSERT INTO social.messages (id, conversation_id, sender_id, body)
VALUES ($1, $2, $3, $4)
RETURNING messages.sent_at
