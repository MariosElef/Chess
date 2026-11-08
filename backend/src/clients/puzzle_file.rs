//! The Lichess puzzle database file, `lichess_db_puzzle.csv.zst`: CSV compressed with zstd.
use std::{fs::File, path::Path};

use anyhow::Context as _;
use serde::Deserialize;

use crate::models::puzzles::puzzle::NewPuzzle;

/// One row of the file. Its last column, the date a puzzle was the daily puzzle, is not used.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct FileRow {
    puzzle_id: String,
    #[serde(rename = "FEN")]
    fen: String,
    moves: String,
    rating: i32,
    rating_deviation: i32,
    popularity: i32,
    nb_plays: i32,
    themes: String,
    game_url: String,
    opening_tags: String,
}

impl FileRow {
    fn into_puzzle(self) -> NewPuzzle {
        NewPuzzle {
            id: self.puzzle_id,
            fen: self.fen,
            moves: self.moves,
            rating: self.rating,
            rating_deviation: self.rating_deviation,
            popularity: self.popularity,
            nb_plays: self.nb_plays,
            themes: self.themes,
            game_url: (!self.game_url.is_empty()).then_some(self.game_url),
            opening_tags: self.opening_tags,
        }
    }
}

/// Reads the packed file `batch_size` puzzles at a time until `deliver` says stop; blocking.
pub fn read_batches(
    path: &Path,
    batch_size: usize,
    mut deliver: impl FnMut(Vec<NewPuzzle>) -> bool,
) -> anyhow::Result<()> {
    let file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let decoder = zstd::Decoder::new(file).context("not a zstd file")?;
    let mut reader = csv::Reader::from_reader(decoder);
    let mut batch = Vec::with_capacity(batch_size);
    for row in reader.deserialize::<FileRow>() {
        batch.push(row.context("a row could not be read")?.into_puzzle());
        if batch.len() == batch_size && !deliver(std::mem::take(&mut batch)) {
            return Ok(());
        }
    }
    if !batch.is_empty() {
        deliver(batch);
    }
    Ok(())
}
