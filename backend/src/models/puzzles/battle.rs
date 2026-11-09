use serde::Serialize;
use uuid::Uuid;

use super::{puzzle::GetPuzzleModel, rating::PuzzleRatingModel};
use crate::{
    enums::puzzles::{BattleResult, BattleStatus},
    models::social::friend::UserRef,
};

/// A battle as one of its players sees it: both scores, the puzzles, the time left, and the result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BattleSnapshot {
    pub id: Uuid,
    pub me: BattleSideModel,
    pub opponent: BattleSideModel,
    pub rated: bool,
    /// The same for both players, easiest first.
    pub puzzles: Vec<GetPuzzleModel>,
    pub duration_ms: u64,
    pub left_ms: u64,
    pub status: BattleStatus,
    pub result: Option<BattleResult>,
    /// How the player's battle rating moved, once a rated battle is over.
    pub rating_change: Option<i32>,
}

/// One side of a battle: who, and how they are doing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BattleSideModel {
    pub id: Uuid,
    pub username: String,
    /// Their picture's hash; none shows their initial.
    pub avatar: Option<String>,
    /// The battle rating from before it; `None` in an unrated battle.
    pub rating: Option<i32>,
    pub score: u32,
    pub mistakes: u32,
}

/// A friend's invitation to a battle, waiting for an answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BattleChallengeModel {
    pub id: Uuid,
    pub from: UserRef,
    pub to: UserRef,
    pub rated: bool,
}

/// The battle rating and this month's wins, for the Puzzle Battle panel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct BattleSummaryModel {
    pub rating: Option<PuzzleRatingModel>,
    pub wins_this_month: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewBattle {
    pub id: Uuid,
    pub first_id: Uuid,
    pub second_id: Uuid,
    pub rated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinishedBattle {
    pub id: Uuid,
    pub first_score: i32,
    pub second_score: i32,
    pub winner_id: Option<Uuid>,
}
