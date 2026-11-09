-- A leaderboard ranks one category's ratings, best first: this keeps that from reading them all.
CREATE INDEX ratings_category_rating_idx ON play.ratings (category_id, rating DESC);
