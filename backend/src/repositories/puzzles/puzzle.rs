use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::puzzles::{
    puzzle::{NewPuzzle, PuzzleRow},
    rating::PuzzleStrength,
};

/// Stores the batch, updating puzzles already stored; returns how many rows were written.
pub async fn upsert_batch(
    db: impl PgExecutor<'_>,
    puzzles: &[NewPuzzle],
) -> Result<u64, sqlx::Error> {
    let ids: Vec<&str> = puzzles.iter().map(|puzzle| puzzle.id.as_str()).collect();
    let fens: Vec<&str> = puzzles.iter().map(|puzzle| puzzle.fen.as_str()).collect();
    let moves: Vec<&str> = puzzles.iter().map(|puzzle| puzzle.moves.as_str()).collect();
    let ratings: Vec<i32> = puzzles.iter().map(|puzzle| puzzle.rating).collect();
    let deviations: Vec<i32> = puzzles
        .iter()
        .map(|puzzle| puzzle.rating_deviation)
        .collect();
    let popularity: Vec<i32> = puzzles.iter().map(|puzzle| puzzle.popularity).collect();
    let plays: Vec<i32> = puzzles.iter().map(|puzzle| puzzle.nb_plays).collect();
    let themes: Vec<&str> = puzzles
        .iter()
        .map(|puzzle| puzzle.themes.as_str())
        .collect();
    let game_urls: Vec<Option<&str>> = puzzles
        .iter()
        .map(|puzzle| puzzle.game_url.as_deref())
        .collect();
    let openings: Vec<&str> = puzzles
        .iter()
        .map(|puzzle| puzzle.opening_tags.as_str())
        .collect();
    let result = sqlx::query_file!(
        "src/repositories/sql/puzzles/puzzle/upsert_batch.sql",
        &ids as &[&str],
        &fens as &[&str],
        &moves as &[&str],
        &ratings,
        &deviations,
        &popularity,
        &plays,
        &themes as &[&str],
        &game_urls as &[Option<&str>],
        &openings as &[&str],
    )
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}

/// The first puzzle rated `min..=max` from (`from_rating`, `from_id`) that `user_id` has not tried.
pub async fn first_from(
    db: impl PgExecutor<'_>,
    (min, max): (i32, i32),
    (from_rating, from_id): (i32, &str),
    user_id: Uuid,
) -> Result<Option<PuzzleRow>, sqlx::Error> {
    sqlx::query_file_as!(
        PuzzleRow,
        "src/repositories/sql/puzzles/puzzle/first_from.sql",
        min,
        max,
        from_rating,
        from_id,
        user_id,
    )
    .fetch_optional(db)
    .await
}

/// The puzzle's rating and deviation, or `None` for one not in the database (from Lichess).
pub async fn strength(
    db: impl PgExecutor<'_>,
    puzzle_id: &str,
) -> Result<Option<PuzzleStrength>, sqlx::Error> {
    sqlx::query_file_as!(
        PuzzleStrength,
        "src/repositories/sql/puzzles/puzzle/strength.sql",
        puzzle_id
    )
    .fetch_optional(db)
    .await
}

/// Whether any puzzle has been imported.
pub async fn any(db: impl PgExecutor<'_>) -> Result<bool, sqlx::Error> {
    sqlx::query_file_scalar!("src/repositories/sql/puzzles/puzzle/any.sql")
        .fetch_one(db)
        .await
}
