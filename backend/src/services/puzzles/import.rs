use std::path::PathBuf;

use anyhow::Context as _;
use tokio::{sync::mpsc, task};

use crate::{
    clients::puzzle_file,
    models::puzzles::puzzle::NewPuzzle,
    repositories::{database::Db, puzzles::puzzle},
};

/// Puzzles written per statement.
const BATCH_SIZE: usize = 2_000;
/// Batches read ahead of the database; the reader waits when this many are queued.
const BATCHES_AHEAD: usize = 4;
/// A progress line every this many puzzles.
const REPORT_EVERY: u64 = 200_000;

/// Loads the Lichess puzzle database file into `puzzle.puzzles`; returns how many were written.
pub async fn import_puzzles(db: &Db, path: PathBuf) -> anyhow::Result<u64> {
    let (sender, mut batches) = mpsc::channel::<Vec<NewPuzzle>>(BATCHES_AHEAD);
    let reading = task::spawn_blocking(move || {
        puzzle_file::read_batches(&path, BATCH_SIZE, |batch| {
            sender.blocking_send(batch).is_ok()
        })
    });
    let mut written = 0;
    while let Some(batch) = batches.recv().await {
        let before = written;
        written += puzzle::upsert_batch(db, &batch).await?;
        if before / REPORT_EVERY != written / REPORT_EVERY {
            tracing::info!(written, "importing puzzles");
        }
    }
    reading.await.context("the file reader stopped")??;
    Ok(written)
}
