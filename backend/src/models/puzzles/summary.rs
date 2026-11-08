use serde::Serialize;

use super::{battle::BattleSummaryModel, rating::PuzzleRatingModel};

/// What the Puzzles menu shows: the puzzle rating, the best rush of each kind, ever and today,
/// and the battle rating with this month's wins. A guest has none of them, and is told so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GetPuzzleSummaryModel {
    pub guest: bool,
    pub rating: Option<PuzzleRatingModel>,
    pub best_rush: BestRushModel,
    /// Each kind's best since midnight.
    pub best_today: BestRushModel,
    pub battle: BattleSummaryModel,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct BestRushModel {
    pub three_minutes: Option<i32>,
    pub five_minutes: Option<i32>,
    pub survival: Option<i32>,
}
