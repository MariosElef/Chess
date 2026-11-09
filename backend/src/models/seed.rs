//! Made-up players and their games, for filling an empty server with something to look at.
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::enums::play::{Color, GameResult};

/// A made-up account: its name, when it joined, and the hash of a password nobody knows.
/// No `Debug`: the hash.
pub struct FakePlayer {
    pub id: Uuid,
    pub username: String,
    pub joined: DateTime<Utc>,
    pub password_hash: String,
}

/// A finished made-up game: its terms, every move, how it ended and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeGame {
    pub id: Uuid,
    pub white_id: Uuid,
    pub black_id: Uuid,
    pub category: &'static str,
    pub initial_s: i32,
    pub increment_s: i32,
    pub moves: Vec<FakeMove>,
    /// The position after the last move.
    pub fen: String,
    pub result: GameResult,
    pub winner: Option<Color>,
    pub white_ms: i32,
    pub black_ms: i32,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

/// One move of a made-up game, with both clocks after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeMove {
    pub uci: String,
    pub white_ms: i32,
    pub black_ms: i32,
    pub played_at: DateTime<Utc>,
}
