SELECT conversations.id
FROM social.conversations
WHERE conversations.direct_key = $1
