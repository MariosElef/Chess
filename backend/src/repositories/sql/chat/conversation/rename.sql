-- The group $1 is called $2 from now on.
UPDATE social.conversations
SET title = $2
WHERE conversations.id = $1 AND conversations.kind = 'group'
