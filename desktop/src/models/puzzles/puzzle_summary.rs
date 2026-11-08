use serde::Deserialize;

use super::{battle::BattleSummary, puzzle_rating::PuzzleRating};

/// What the Puzzles menu shows: the puzzle rating, the best rush of each kind, ever and today,
/// and the battle rating with this month's wins. A guest has none of them.
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
