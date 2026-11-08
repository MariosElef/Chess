-- Groups and game chats the player started live on for their other members.
UPDATE social.conversations
SET created_by_id = NULL
WHERE conversations.created_by_id = $1
