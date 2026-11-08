SELECT conversation_members.user_id
FROM social.conversation_members
WHERE conversation_members.conversation_id = $1
