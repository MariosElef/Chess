use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::account::{CategoryRatingRow, GameResultsRow, HistoryRow, PuzzleCountsRow};

/// Every rating category, in order, with the player's rating in it and their best, if any.
pub async fn ratings(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<Vec<CategoryRatingRow>, sqlx::Error> {
    sqlx::query_file_as!(
        CategoryRatingRow,
        "src/repositories/sql/account/stats/ratings.sql",
        user_id
    )
    .fetch_all(db)
    .await
}

/// The player's latest `count` ratings in each category but `skipped`, oldest first.
pub async fn history(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    (count, skipped): (i64, &str),
) -> Result<Vec<HistoryRow>, sqlx::Error> {
    sqlx::query_file_as!(
        HistoryRow,
        "src/repositories/sql/account/stats/history.sql",
        user_id,
        count,
        skipped
    )
    .fetch_all(db)
    .await
}

/// The player's finished games, won and drawn, per category.
pub async fn game_results(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<Vec<GameResultsRow>, sqlx::Error> {
    sqlx::query_file_as!(
        GameResultsRow,
        "src/repositories/sql/account/stats/game_results.sql",
        user_id
    )
    .fetch_all(db)
    .await
}

pub async fn puzzle_counts(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
) -> Result<PuzzleCountsRow, sqlx::Error> {
    sqlx::query_file_as!(
        PuzzleCountsRow,
        "src/repositories/sql/account/stats/puzzle_counts.sql",
        user_id
    )
    .fetch_one(db)
    .await
}
