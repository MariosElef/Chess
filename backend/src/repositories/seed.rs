//! Writes made-up players and games straight in, with the times they are meant to have happened.
use chrono::{DateTime, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{
    enums::play::Color,
    models::{
        rating::RatingRow,
        seed::{FakeGame, FakePlayer},
    },
};

/// Whether the account went in; `false` when its username was taken.
pub async fn insert_player(
    db: impl PgExecutor<'_>,
    player: &FakePlayer,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query_file!(
        "src/repositories/sql/seed/insert_player.sql",
        player.id,
        player.username,
        player.password_hash,
        player.joined,
    )
    .execute(db)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn insert_game(db: impl PgExecutor<'_>, game: &FakeGame) -> Result<(), sqlx::Error> {
    let plies = i32::try_from(game.moves.len()).unwrap_or(i32::MAX);
    sqlx::query_file!(
        "src/repositories/sql/seed/insert_game.sql",
        game.id,
        game.white_id,
        game.black_id,
        game.category,
        game.initial_s,
        game.increment_s,
        game.result.key(),
        game.winner.map(Color::key),
        plies,
        game.fen,
        game.white_ms,
        game.black_ms,
        game.started_at,
        game.finished_at,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn insert_moves(db: impl PgExecutor<'_>, game: &FakeGame) -> Result<(), sqlx::Error> {
    let ucis: Vec<String> = game.moves.iter().map(|mv| mv.uci.clone()).collect();
    let white: Vec<i32> = game.moves.iter().map(|mv| mv.white_ms).collect();
    let black: Vec<i32> = game.moves.iter().map(|mv| mv.black_ms).collect();
    let played: Vec<DateTime<Utc>> = game.moves.iter().map(|mv| mv.played_at).collect();
    sqlx::query_file!(
        "src/repositories/sql/seed/insert_moves.sql",
        game.id,
        &ucis,
        &white,
        &black,
        &played,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Sets the player's rating in `category` outright, as standing for `results` results.
pub async fn set_rating(
    db: impl PgExecutor<'_>,
    (user_id, category): (Uuid, &str),
    rating: RatingRow,
    results: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query_file!(
        "src/repositories/sql/seed/set_rating.sql",
        user_id,
        category,
        rating.rating,
        rating.deviation,
        rating.volatility,
        results,
    )
    .execute(db)
    .await?;
    Ok(())
}
