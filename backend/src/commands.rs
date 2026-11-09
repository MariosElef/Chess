//! What the server binary does besides serving: `chess-backend import-puzzles <file>` and
//! `chess-backend seed-fake-players <count>`.
use std::path::PathBuf;

use crate::{
    config::AppConfig,
    repositories::database,
    services::{puzzles::import, seed},
};

/// Imports the Lichess puzzle database file into the configured database, migrating it first.
pub async fn import_puzzles(config: &AppConfig, path: PathBuf) -> anyhow::Result<()> {
    let db = database::connect(&config.database_url).await?;
    database::migrate(&db).await?;
    let written = import::import_puzzles(&db, path).await?;
    tracing::info!(written, "puzzles imported");
    Ok(())
}

/// Creates `count` made-up players with pictures, games and ratings, migrating the database first.
pub async fn seed_fake_players(config: &AppConfig, count: usize) -> anyhow::Result<()> {
    let db = database::connect(&config.database_url).await?;
    database::migrate(&db).await?;
    let report = seed::seed_fake_players(&db, count).await?;
    tracing::info!(
        players = report.players,
        pictures = report.pictures,
        games = report.games,
        "fake players seeded"
    );
    Ok(())
}
