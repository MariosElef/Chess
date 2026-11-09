use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::play::history::PlayedGameRow;

/// Up to `limit` of the player's finished games before `before`, the latest first.
pub async fn page(
    db: impl PgExecutor<'_>,
    player: Uuid,
    (before, limit): (Option<Uuid>, i64),
) -> Result<Vec<PlayedGameRow>, sqlx::Error> {
    sqlx::query_file_as!(
        PlayedGameRow,
        "src/repositories/sql/play/history/page.sql",
        player,
        before,
        limit
    )
    .fetch_all(db)
    .await
}

/// The finished game, if the player played in it.
pub async fn one(
    db: impl PgExecutor<'_>,
    player: Uuid,
    game_id: Uuid,
) -> Result<Option<PlayedGameRow>, sqlx::Error> {
    sqlx::query_file_as!(
        PlayedGameRow,
        "src/repositories/sql/play/history/one.sql",
        player,
        game_id
    )
    .fetch_optional(db)
    .await
}

/// The game's moves in UCI, in the order they were played.
pub async fn moves(db: impl PgExecutor<'_>, game_id: Uuid) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_file_scalar!("src/repositories/sql/play/history/moves.sql", game_id)
        .fetch_all(db)
        .await
}
