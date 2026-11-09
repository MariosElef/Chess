-- Before ownership passed on, a group whose owner left had none: its longest-standing member
-- owns it now.
UPDATE social.conversation_members
SET role = 'owner'
WHERE
    (conversation_members.conversation_id, conversation_members.user_id) IN (
        SELECT DISTINCT ON (conversation_members.conversation_id)
            conversation_members.conversation_id,
            conversation_members.user_id
        FROM social.conversation_members
        INNER JOIN social.conversations
            ON conversations.id = conversation_members.conversation_id
        WHERE
            conversations.kind = 'group'
            AND NOT EXISTS (
                SELECT 1
                FROM social.conversation_members
                WHERE
                    conversation_members.conversation_id = conversations.id
                    AND conversation_members.role = 'owner'
            )
        ORDER BY
            conversation_members.conversation_id,
            conversation_members.joined_at,
            conversation_members.user_id
    )
