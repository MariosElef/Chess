-- $1's picture is now the JPEG $3, named by its hash $2; any earlier one is replaced.
INSERT INTO account.avatars (user_id, hash, image)
VALUES ($1, $2, $3)
ON CONFLICT (user_id) DO UPDATE
SET hash = excluded.hash, image = excluded.image, updated_at = now()
