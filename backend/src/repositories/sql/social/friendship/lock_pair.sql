-- Holds the pair ($1, $2) until the transaction ends, so a request and a block between them take
-- turns.
SELECT pg_advisory_xact_lock(hashtextextended($1::uuid::text || ':' || $2::uuid::text, 0))
