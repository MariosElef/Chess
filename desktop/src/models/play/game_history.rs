use serde::Deserialize;

use crate::enums::play::{Color, GameResult, PlayerResult};

/// One of the player's finished online games, as their history lists it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlayedGame {
    pub id: String,
    /// The side the player had.
    pub color: Color,
    pub opponent: String,
    pub result: PlayerResult,
    pub outcome: GameResult,
    /// The server's key for the speed: `bullet`, `blitz`, `rapid` or `classical`.
    pub category: String,
    pub initial_s: u32,
    pub increment_s: u32,
    pub rated: bool,
    /// Half-moves played.
    pub plies: u32,
    /// Seconds since it ended.
    pub ago_s: i64,
    pub rating: Option<i32>,
    pub rating_change: Option<i32>,
}

/// A finished game with its moves, to open on the analysis board.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GameRecord {
    #[serde(flatten)]
    pub game: PlayedGame,
    /// In UCI, from the starting position.
    pub moves: Vec<String>,
}
