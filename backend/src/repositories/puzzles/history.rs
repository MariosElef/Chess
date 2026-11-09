use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::puzzles::history::{AttemptRow, BattleRow};

/// Up to `limit` of the player's finished battles before `before`, the latest first.
pub async fn battles(
    db: impl PgExecutor<'_>,
    player: Uuid,
    (before, limit): (Option<Uuid>, i64),
) -> Result<Vec<BattleRow>, sqlx::Error> {
    sqlx::query_file_as!(
        BattleRow,
        "src/repositories/sql/puzzles/history/battles.sql",
        player,
        before,
        limit
    )
    .fetch_all(db)
    .await
}

/// Up to `limit` of the player's puzzle attempts before `before`, the latest first.
pub async fn attempts(
    db: impl PgExecutor<'_>,
    player: Uuid,
    (before, limit): (Option<Uuid>, i64),
) -> Result<Vec<AttemptRow>, sqlx::Error> {
    sqlx::query_file_as!(
        AttemptRow,
        "src/repositories/sql/puzzles/history/attempts.sql",
        player,
        before,
        limit
    )
    .fetch_all(db)
    .await
}
