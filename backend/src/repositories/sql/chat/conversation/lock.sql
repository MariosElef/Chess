-- Holds the conversation $1 until the transaction ends, so its members change one at a time;
-- messages still go in meanwhile.
SELECT conversations.id
FROM social.conversations
WHERE conversations.id = $1
FOR NO KEY UPDATE
