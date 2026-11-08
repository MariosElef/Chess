use serde::Deserialize;

use super::{puzzle_rating::PuzzleRating, puzzle_text::PuzzleText};
use crate::{
    enums::puzzles::{BattleResult, BattleStatus},
    models::social::UserRef,
};

/// A Puzzle Battle as the server sends it: both sides, the puzzles, time left, and the end.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BattleSnapshot {
    pub id: String,
    pub me: BattleSide,
    pub opponent: BattleSide,
    pub rated: bool,
    pub puzzles: Vec<PuzzleText>,
    pub left_ms: u64,
    pub status: BattleStatus,
    pub result: Option<BattleResult>,
    pub rating_change: Option<i32>,
}

/// One side of a battle: who, and how they are doing.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BattleSide {
    pub username: String,
    pub rating: Option<i32>,
    pub score: u32,
    pub mistakes: u32,
}

/// A friend's invitation to a battle, waiting for an answer.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BattleChallenge {
    pub id: String,
    pub from: UserRef,
    pub to: UserRef,
    pub rated: bool,
}

/// The battle rating and this month's wins, for the Puzzle Battle panel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct BattleSummary {
    pub rating: Option<PuzzleRating>,
    pub wins_this_month: u32,
}
