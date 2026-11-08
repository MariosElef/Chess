use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use crate::models::rating::{RatingRow, RatingSource};

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
/// names the game or puzzle attempt that moved it.
pub async fn record_result(
    db: &mut PgConnection,
    user_id: Uuid,
    category: &str,
    source: RatingSource,
    rating: RatingRow,
) -> Result<(), sqlx::Error> {
    let (game_id, attempt_id) = match source {
        RatingSource::Game(game_id) => (Some(game_id), None),
        RatingSource::PuzzleAttempt(attempt_id) => (None, Some(attempt_id)),
    };
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
        game_id,
        attempt_id,
        rating.rating,
        rating.deviation,
        rating.volatility,
    )
    .execute(db)
    .await?;
    Ok(())
}
