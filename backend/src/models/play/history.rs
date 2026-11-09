use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::play::{Color, GameResult, PlayerResult};

/// A page of history: the latest, or what came before `before`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct HistoryQuery {
    pub before: Option<Uuid>,
}

/// A finished game as stored, with both players' names and how long ago it ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayedGameRow {
    pub id: Uuid,
    pub white_id: Uuid,
    pub white_name: String,
    pub black_name: String,
    pub category: String,
    pub initial_s: i32,
    pub increment_s: i32,
    pub rated: bool,
    pub outcome: String,
    pub winner: Option<String>,
    pub ply_count: i32,
    pub ago_s: i64,
}

/// One of the player's finished games, as their history lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayedGameModel {
    pub id: Uuid,
    /// The side the player had.
    pub color: Color,
    pub opponent: String,
    pub result: PlayerResult,
    pub outcome: GameResult,
    pub category: String,
    pub initial_s: i32,
    pub increment_s: i32,
    pub rated: bool,
    /// Half-moves played.
    pub plies: i32,
    /// Seconds since it ended.
    pub ago_s: i64,
    /// The player's rating after it and how far it moved, for a rated game.
    pub rating: Option<i32>,
    pub rating_change: Option<i32>,
}

/// A finished game with its moves, to open on the analysis board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GameRecordModel {
    #[serde(flatten)]
    pub game: PlayedGameModel,
    /// In UCI, from the starting position.
    pub moves: Vec<String>,
}
