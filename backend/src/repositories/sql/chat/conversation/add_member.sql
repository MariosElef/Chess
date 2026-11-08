INSERT INTO social.conversation_members (conversation_id, user_id, role)
VALUES ($1, $2, $3)
ON CONFLICT (conversation_id, user_id) DO NOTHING
