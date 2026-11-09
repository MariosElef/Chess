-- Whether $2 owns the conversation $1.
SELECT EXISTS (
    SELECT 1
    FROM social.conversation_members
    WHERE
        conversation_members.conversation_id = $1
        AND conversation_members.user_id = $2
        AND conversation_members.role = 'owner'
) AS "owner!"
