use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use crate::models::puzzles::rating::RatingRow;

/// The player's rating in `category`, or `None` before their first result in it.
pub async fn get(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    category: &str,
) -> Result<Option<RatingRow>, sqlx::Error> {
    sqlx::query_file_as!(
        RatingRow,
        "src/repositories/sql/rating/get.sql",
        user_id,
        category
    )
    .fetch_optional(db)
    .await
}

/// As `get`, locked for the caller's transaction.
pub async fn get_for_update(
    db: &mut PgConnection,
    user_id: Uuid,
    category: &str,
) -> Result<Option<RatingRow>, sqlx::Error> {
    sqlx::query_file_as!(
        RatingRow,
        "src/repositories/sql/rating/get_for_update.sql",
        user_id,
        category
    )
    .fetch_optional(db)
    .await
}

/// Stores the new rating, counting one more result, and its row in the rating history, which
/// names the puzzle attempt that moved it.
pub async fn record_puzzle_result(
    db: &mut PgConnection,
    user_id: Uuid,
    category: &str,
    attempt_id: Uuid,
    rating: RatingRow,
) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/rating/upsert.sql",
        user_id,
        category,
        rating.rating,
        rating.deviation,
        rating.volatility,
    )
    .execute(&mut *db)
    .await?;
    sqlx::query_file!(
        "src/repositories/sql/rating/insert_history.sql",
        Uuid::now_v7(),
        user_id,
        category,
        attempt_id,
        rating.rating,
        rating.deviation,
        rating.volatility,
    )
    .execute(db)
    .await?;
    Ok(())
}
