-- A group the player was the last member of has no one left to read it. A game's chat stays
-- even when empty: its game keeps pointing at it.
DELETE FROM social.conversations
WHERE
    conversations.kind = 'group'
    AND NOT EXISTS (
        SELECT 1
        FROM social.conversation_members
        WHERE conversation_members.conversation_id = conversations.id
    )
