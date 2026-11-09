-- Every conversation $1 is in or started, held in id order until the transaction ends, so no
-- member change runs alongside and ownership can pass on.
SELECT conversations.id
FROM social.conversations
WHERE
    conversations.created_by_id = $1
    OR conversations.id IN (
        SELECT conversation_members.conversation_id
        FROM social.conversation_members
        WHERE conversation_members.user_id = $1
    )
ORDER BY conversations.id
FOR NO KEY UPDATE
