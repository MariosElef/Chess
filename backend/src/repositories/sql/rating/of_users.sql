-- Every rating the players have, in any category.
SELECT
    ratings.user_id,
    ratings.category_id,
    ratings.rating
FROM play.ratings
WHERE ratings.user_id = ANY($1)
