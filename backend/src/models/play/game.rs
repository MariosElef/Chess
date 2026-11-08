use serde::Serialize;
use uuid::Uuid;

use super::player::PlayerModel;
use crate::enums::play::{Color, GameResult, GameStatus};

/// A live game as both players see it, sent whole after every change: the moves from the
/// start, both clocks at the moment it was sent, and how it stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GameSnapshot {
    pub id: Uuid,
    pub white: PlayerModel,
    pub black: PlayerModel,
    pub rated: bool,
    pub initial_s: u32,
    pub increment_s: u32,
    /// In UCI, from the starting position.
    pub moves: Vec<String>,
    pub white_ms: u64,
    pub black_ms: u64,
    /// The clock counting down now, if any: none before the first move and once it is over.
    pub running: Option<Color>,
    pub status: GameStatus,
    pub result: Option<GameResult>,
    pub winner: Option<Color>,
    pub draw_offer: Option<Color>,
    /// How each rating moved, once a rated game is over.
    pub rating_changes: Option<RatingChanges>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RatingChanges {
    pub white: i32,
    pub black: i32,
}

/// A game to store as it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGame {
    pub id: Uuid,
    pub white_id: Uuid,
    pub black_id: Uuid,
    pub category: String,
    pub initial_s: i32,
    pub increment_s: i32,
    pub rated: bool,
    pub fen: String,
}

/// A move to store, with the game as it stands after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayedMove {
    pub game_id: Uuid,
    pub ply: i32,
    pub uci: String,
    pub fen: String,
    pub white_ms: i32,
    pub black_ms: i32,
}

/// How a game ended, to store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinishedGame {
    pub game_id: Uuid,
    pub result: GameResult,
    pub winner: Option<Color>,
    pub white_ms: i32,
    pub black_ms: i32,
}
