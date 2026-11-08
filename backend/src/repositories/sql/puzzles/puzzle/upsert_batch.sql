-- Themes and opening tags arrive space-separated, as in the Lichess file, and are split here.
INSERT INTO puzzle.puzzles (
    id, fen, moves, rating, rating_deviation, popularity, nb_plays, themes, game_url,
    opening_tags
)
SELECT
    batch.id,
    batch.fen,
    batch.moves,
    batch.rating,
    batch.rating_deviation,
    batch.popularity,
    batch.nb_plays,
    string_to_array(batch.themes, ' '),
    batch.game_url,
    string_to_array(batch.opening_tags, ' ')
FROM unnest(
    $1::text [], $2::text [], $3::text [], $4::int [], $5::int [], $6::int [], $7::int [],
    $8::text [], $9::text [], $10::text []
) AS batch (
    id, fen, moves, rating, rating_deviation, popularity, nb_plays, themes, game_url,
    opening_tags
)
ON CONFLICT (id) DO UPDATE SET
    fen = excluded.fen,
    moves = excluded.moves,
    rating = excluded.rating,
    rating_deviation = excluded.rating_deviation,
    popularity = excluded.popularity,
    nb_plays = excluded.nb_plays,
    themes = excluded.themes,
    game_url = excluded.game_url,
    opening_tags = excluded.opening_tags
