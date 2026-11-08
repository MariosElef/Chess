-- $1's conversations but games' chats, the most recently active first, with how many messages
-- from others they have not read.
SELECT
    conversations.id,
    conversations.kind,
    conversations.title,
    (
        SELECT count(*)
        FROM social.messages
        WHERE
            messages.conversation_id = conversations.id
            AND messages.sender_id <> $1
            AND (
                conversation_members.last_read_at IS NULL
                OR messages.sent_at > conversation_members.last_read_at
            )
    ) AS "unread!"
FROM social.conversations
INNER JOIN social.conversation_members
    ON
        conversation_members.conversation_id = conversations.id
        AND conversation_members.user_id = $1
WHERE conversations.kind <> 'game'
ORDER BY
    coalesce(
        (
            SELECT max(messages.sent_at)
            FROM social.messages
            WHERE messages.conversation_id = conversations.id
        ),
        conversations.created_at
    ) DESC
