use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{
    enums::play::Color,
    models::play::game::{FinishedGame, NewGame, PlayedMove, RematchTerms},
};

pub async fn insert(db: impl PgExecutor<'_>, game: &NewGame) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/play/game/insert.sql",
        game.id,
        game.white_id,
        game.black_id,
        game.category,
        game.initial_s,
        game.increment_s,
        game.rated,
        game.fen,
        game.conversation_id,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn record_move(db: impl PgExecutor<'_>, played: &PlayedMove) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/play/game/record_move.sql",
        played.game_id,
        played.ply,
        played.uci,
        played.fen,
        played.white_ms,
        played.black_ms,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// The side whose draw offer is open, or `None` once it is answered.
pub async fn set_draw_offer(
    db: impl PgExecutor<'_>,
    game_id: Uuid,
    offered_by: Option<Color>,
) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/play/game/set_draw_offer.sql",
        game_id,
        offered_by.map(Color::key),
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn finish(db: impl PgExecutor<'_>, finished: &FinishedGame) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/play/game/finish.sql",
        finished.game_id,
        finished.result.key(),
        finished.winner.map(Color::key),
        finished.white_ms,
        finished.black_ms,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Calls a game off before both sides moved.
pub async fn abort(db: impl PgExecutor<'_>, game_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query_file!("src/repositories/sql/play/game/abort.sql", game_id)
        .execute(db)
        .await?;
    Ok(())
}

/// Closes every game still marked as running; returns how many.
pub async fn abort_unfinished(db: impl PgExecutor<'_>) -> Result<u64, sqlx::Error> {
    let result = sqlx::query_file!("src/repositories/sql/play/game/abort_unfinished.sql")
        .execute(db)
        .await?;
    Ok(result.rows_affected())
}

/// The game a rematch would replay: the player was in it, and it ended in the last ten minutes.
pub async fn rematch_terms(
    db: impl PgExecutor<'_>,
    game_id: Uuid,
    player: Uuid,
) -> Result<Option<RematchTerms>, sqlx::Error> {
    sqlx::query_file_as!(
        RematchTerms,
        "src/repositories/sql/play/game/rematch_terms.sql",
        game_id,
        player
    )
    .fetch_optional(db)
    .await
}

/// Whether the player is still who played the game: signed in on a session from before it ended.
pub async fn seated_since(
    db: impl PgExecutor<'_>,
    game_id: Uuid,
    player: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_file_scalar!(
        "src/repositories/sql/play/game/seated_since.sql",
        game_id,
        player
    )
    .fetch_one(db)
    .await
}
