use serde::Serialize;

use super::rating::PuzzleRatingModel;

/// What the Puzzles menu shows: the puzzle rating and the best rush of each kind. A guest has
/// neither, and is told so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GetPuzzleSummaryModel {
    pub guest: bool,
    pub rating: Option<PuzzleRatingModel>,
    pub best_rush: BestRushModel,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct BestRushModel {
    pub three_minutes: Option<i32>,
    pub five_minutes: Option<i32>,
    pub survival: Option<i32>,
}
