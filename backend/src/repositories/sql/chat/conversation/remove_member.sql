DELETE FROM social.conversation_members
WHERE conversation_members.conversation_id = $1 AND conversation_members.user_id = $2
