use serde::Deserialize;

use crate::enums::play::PlayerResult;

/// One of the player's finished puzzle battles, as their history lists it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlayedBattle {
    pub id: String,
    pub opponent: String,
    pub my_score: u32,
    pub their_score: u32,
    pub result: PlayerResult,
    pub rated: bool,
    /// Seconds since it ended.
    pub ago_s: i64,
    pub rating: Option<i32>,
    pub rating_change: Option<i32>,
}

/// One of the player's puzzle attempts, as their history lists it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlayedAttempt {
    pub id: String,
    pub puzzle_id: String,
    pub puzzle_rating: Option<i32>,
    pub solved: bool,
    pub duration_ms: u32,
    pub rated: bool,
    /// Seconds since it.
    pub ago_s: i64,
    pub rating: Option<i32>,
    pub rating_change: Option<i32>,
}
