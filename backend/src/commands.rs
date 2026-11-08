//! What the server binary does besides serving: `chess-backend import-puzzles <file>`.
use std::path::PathBuf;

use crate::{config::AppConfig, repositories::database, services::puzzles::import};

/// Imports the Lichess puzzle database file into the configured database, migrating it first.
pub async fn import_puzzles(config: &AppConfig, path: PathBuf) -> anyhow::Result<()> {
    let db = database::connect(&config.database_url).await?;
    database::migrate(&db).await?;
    let written = import::import_puzzles(&db, path).await?;
    tracing::info!(written, "puzzles imported");
    Ok(())
}
