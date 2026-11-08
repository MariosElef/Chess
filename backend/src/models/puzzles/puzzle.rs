use serde::{Deserialize, Serialize};

/// A puzzle for the app: the position the player moves from, the opponent's move that led to
/// it, and the solution in UCI, the player's moves and the opponent's answers in turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GetPuzzleModel {
    pub id: String,
    pub rating: i32,
    pub plays: i32,
    pub themes: Vec<String>,
    pub fen: String,
    pub last_move: Option<String>,
    pub solution: Vec<String>,
}

/// The rating range a player asks puzzles from; either end may be left out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct PuzzleQuery {
    pub min_rating: Option<i32>,
    pub max_rating: Option<i32>,
}

/// A puzzle as stored: as the Lichess database has it, the position before the opponent's
/// move and the moves from there, the opponent's first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PuzzleRow {
    pub id: String,
    pub fen: String,
    pub moves: String,
    pub rating: i32,
    pub nb_plays: i32,
    pub themes: Vec<String>,
}

/// One puzzle read from the Lichess database file, to store or update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPuzzle {
    pub id: String,
    pub fen: String,
    pub moves: String,
    pub rating: i32,
    pub rating_deviation: i32,
    pub popularity: i32,
    pub nb_plays: i32,
    /// Space-separated, as in the file.
    pub themes: String,
    pub game_url: Option<String>,
    /// Space-separated, as in the file; often empty.
    pub opening_tags: String,
}
