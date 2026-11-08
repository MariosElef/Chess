-- Lichess's boundaries on the estimated game length, initial_s + 40 * increment_s.
INSERT INTO play.rating_categories (id, min_estimate_s, max_estimate_s, sort_order) VALUES
('bullet', 0, 179, 1),
('blitz', 180, 479, 2),
('rapid', 480, 1499, 3),
('classical', 1500, NULL, 4),
('puzzle', NULL, NULL, 5);
