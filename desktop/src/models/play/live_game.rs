use serde::Deserialize;

use crate::enums::play::{Color, GameResult, LiveStatus};

/// An online game as the server sends it after every change: the moves from the start, both
/// clocks at the moment it was sent, and how it stands.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GameSnapshot {
    pub id: String,
    pub white: PlayerAtBoard,
    pub black: PlayerAtBoard,
    pub rated: bool,
    pub initial_s: u32,
    pub increment_s: u32,
    /// In UCI, from the starting position.
    pub moves: Vec<String>,
    pub white_ms: u64,
    pub black_ms: u64,
    /// The clock counting down when it was sent, if any.
    pub running: Option<Color>,
    pub status: LiveStatus,
    pub result: Option<GameResult>,
    pub winner: Option<Color>,
    pub draw_offer: Option<Color>,
    pub rating_changes: Option<RatingChanges>,
}

/// A player at the board: their name, and their rating unless they are a guest.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlayerAtBoard {
    pub id: String,
    pub username: String,
    pub rating: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct RatingChanges {
    pub white: i32,
    pub black: i32,
}
