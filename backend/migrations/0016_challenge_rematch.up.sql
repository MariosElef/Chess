-- A challenge's record says which finished game it asks to play again, for a rematch.
ALTER TABLE play.challenges ADD COLUMN rematch_of uuid;
ALTER TABLE play.challenges ADD CONSTRAINT challenges_rematch_of_fkey
FOREIGN KEY (rematch_of) REFERENCES play.games (id);
