-- A group $1 left without an owner is owned by its longest-standing member, if any is left;
-- other conversations have no owner.
UPDATE social.conversation_members
SET role = 'owner'
WHERE
    conversation_members.conversation_id = $1
    AND conversation_members.user_id = (
        SELECT conversation_members.user_id
        FROM social.conversation_members
        WHERE conversation_members.conversation_id = $1
        ORDER BY conversation_members.joined_at, conversation_members.user_id
        LIMIT 1
    )
    AND NOT EXISTS (
        SELECT 1
        FROM social.conversation_members
        WHERE conversation_members.conversation_id = $1 AND conversation_members.role = 'owner'
    )
    AND EXISTS (
        SELECT 1
        FROM social.conversations
        WHERE conversations.id = $1 AND conversations.kind = 'group'
    )
