ALTER TABLE play.games ADD COLUMN public boolean NOT NULL DEFAULT true;
ALTER TABLE play.rating_categories ADD COLUMN min_estimate_s integer, ADD COLUMN max_estimate_s integer;
UPDATE play.rating_categories SET min_estimate_s = 0, max_estimate_s = 179 WHERE id = 'bullet';
UPDATE play.rating_categories SET min_estimate_s = 180, max_estimate_s = 479 WHERE id = 'blitz';
UPDATE play.rating_categories SET min_estimate_s = 480, max_estimate_s = 1499 WHERE id = 'rapid';
UPDATE play.rating_categories SET min_estimate_s = 1500 WHERE id = 'classical';
