-- A direct chat is the pair's alone: it goes with its members and messages.
DELETE FROM social.conversations
WHERE
    conversations.kind = 'direct'
    AND conversations.id IN (
        SELECT conversation_members.conversation_id
        FROM social.conversation_members
        WHERE conversation_members.user_id = $1
    )
