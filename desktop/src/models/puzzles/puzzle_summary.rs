use serde::Deserialize;

use super::puzzle_rating::PuzzleRating;

/// What the Puzzles menu shows: the puzzle rating and the best rush of each kind. A guest has
/// neither.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct PuzzleSummary {
    pub guest: bool,
    pub rating: Option<PuzzleRating>,
    pub best_rush: BestRush,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct BestRush {
    pub three_minutes: Option<i32>,
    pub five_minutes: Option<i32>,
    pub survival: Option<i32>,
}
