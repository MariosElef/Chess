-- Everyone who shares a chat with the player: they hear their chats changed once it closes.
SELECT DISTINCT conversation_members.user_id
FROM social.conversation_members
WHERE
    conversation_members.user_id <> $1
    AND conversation_members.conversation_id IN (
        SELECT conversation_members.conversation_id
        FROM social.conversation_members
        WHERE conversation_members.user_id = $1
    )
