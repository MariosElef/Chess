use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::puzzles::battle::{FinishedBattle, NewBattle};

/// Stores a battle as it starts.
pub async fn insert(db: impl PgExecutor<'_>, battle: &NewBattle) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/puzzles/battle/insert.sql",
        battle.id,
        battle.first_id,
        battle.second_id,
        battle.rated,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Stores how it ended: both scores and the winner, `None` for a draw.
pub async fn finish(db: impl PgExecutor<'_>, battle: &FinishedBattle) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/puzzles/battle/finish.sql",
        battle.id,
        battle.first_score,
        battle.second_score,
        battle.winner_id,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The battles the player has won since the month began.
pub async fn wins_this_month(db: impl PgExecutor<'_>, user_id: Uuid) -> Result<i64, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/puzzles/battle/wins_this_month.sql",
        user_id
    )
    .fetch_one(db)
    .await
}

/// Closes every battle still marked as running; returns how many.
pub async fn close_unfinished(db: impl PgExecutor<'_>) -> Result<u64, sqlx::Error> {
    let result = sqlx::query_file!("src/repositories/sql/puzzles/battle/close_unfinished.sql")
        .execute(db)
        .await?;
    Ok(result.rows_affected())
}
