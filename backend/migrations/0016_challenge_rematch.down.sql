-- Challenge records stop saying which game a rematch replayed.
ALTER TABLE play.challenges DROP COLUMN rematch_of;
