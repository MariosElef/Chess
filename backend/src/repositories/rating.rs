use sqlx::{PgConnection, PgExecutor};
use uuid::Uuid;

use crate::models::rating::{RatingChangeRow, RatingRow, RatingSource, UserRatingRow};

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

/// Holds the players' rows for the caller's transaction, so none of them closes meanwhile.
pub async fn lock_players(db: &mut PgConnection, players: &[Uuid]) -> Result<(), sqlx::Error> {
    sqlx::query_file_scalar!("src/repositories/sql/rating/lock_players.sql", players)
        .fetch_all(db)
        .await?;
    Ok(())
}

/// Every rating the players have, in any category.
pub async fn of_users(
    db: impl PgExecutor<'_>,
    players: &[Uuid],
) -> Result<Vec<UserRatingRow>, sqlx::Error> {
    sqlx::query_file_as!(
        UserRatingRow,
        "src/repositories/sql/rating/of_users.sql",
        players
    )
    .fetch_all(db)
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

/// Stores the new rating, one more result, and a history row naming what moved it.
pub async fn record_result(
    db: &mut PgConnection,
    user_id: Uuid,
    category: &str,
    source: RatingSource,
    rating: RatingRow,
) -> Result<(), sqlx::Error> {
    let (game_id, attempt_id, battle_id) = match source {
        RatingSource::Game(game_id) => (Some(game_id), None, None),
        RatingSource::PuzzleAttempt(attempt_id) => (None, Some(attempt_id), None),
        RatingSource::Battle(battle_id) => (None, None, Some(battle_id)),
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
        battle_id,
        rating.rating,
        rating.deviation,
        rating.volatility,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The player's rating after each of `sources` (games, puzzle attempts or battles), and before.
pub async fn changes(
    db: impl PgExecutor<'_>,
    user_id: Uuid,
    sources: &[Uuid],
) -> Result<Vec<RatingChangeRow>, sqlx::Error> {
    sqlx::query_file_as!(
        RatingChangeRow,
        "src/repositories/sql/rating/changes.sql",
        user_id,
        sources
    )
    .fetch_all(db)
    .await
}
