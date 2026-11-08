use serde::Deserialize;

use super::{battle::BattleSummary, puzzle_rating::PuzzleRating};

/// What the Puzzles menu shows: puzzle rating, best rushes and battle rating; none for guests.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct PuzzleSummary {
    pub guest: bool,
    pub rating: Option<PuzzleRating>,
    pub best_rush: BestRush,
    /// Each kind's best since midnight.
    pub best_today: BestRush,
    pub battle: BattleSummary,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct BestRush {
    pub three_minutes: Option<i32>,
    pub five_minutes: Option<i32>,
    pub survival: Option<i32>,
}
