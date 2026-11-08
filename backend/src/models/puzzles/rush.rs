use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::enums::puzzles::RushKind;

/// A finished Puzzle Rush.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct CreateRushModel {
    pub kind: RushKind,
    pub score: i32,
}

/// The player's best of that kind, and whether this run set it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GetRushModel {
    pub best: i32,
    pub new_best: bool,
}

/// A rush to store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewRushRun {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: RushKind,
    pub score: i32,
}

/// A best score, as the database groups them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BestRushRow {
    pub kind: String,
    pub best: i32,
}
