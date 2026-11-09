use serde::Serialize;
use uuid::Uuid;

use crate::enums::play::PlayerResult;

/// A finished battle as stored, with the other player's name and how long ago it ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleRow {
    pub id: Uuid,
    pub first_id: Uuid,
    pub opponent: String,
    pub first_score: i32,
    pub second_score: i32,
    pub winner_id: Option<Uuid>,
    pub rated: bool,
    pub ago_s: i64,
}

/// One of the player's finished battles, as their history lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayedBattleModel {
    pub id: Uuid,
    pub opponent: String,
    pub my_score: i32,
    pub their_score: i32,
    pub result: PlayerResult,
    pub rated: bool,
    /// Seconds since it ended.
    pub ago_s: i64,
    /// The player's battle rating after it and how far it moved, for a rated battle.
    pub rating: Option<i32>,
    pub rating_change: Option<i32>,
}

/// A puzzle attempt as stored, with the puzzle's rating and how long ago it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptRow {
    pub id: Uuid,
    pub puzzle_id: String,
    pub puzzle_rating: Option<i32>,
    pub solved: bool,
    pub duration_ms: i32,
    pub rated: bool,
    pub ago_s: i64,
}

/// One of the player's puzzle attempts, as their history lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlayedAttemptModel {
    pub id: Uuid,
    pub puzzle_id: String,
    pub puzzle_rating: Option<i32>,
    pub solved: bool,
    pub duration_ms: i32,
    pub rated: bool,
    /// Seconds since it.
    pub ago_s: i64,
    /// The player's puzzle rating after it and how far it moved, for a rated attempt.
    pub rating: Option<i32>,
    pub rating_change: Option<i32>,
}
