UPDATE social.conversation_members
SET last_read_at = now()
WHERE conversation_members.conversation_id = $1 AND conversation_members.user_id = $2
