use serde::Serialize;

/// A puzzle rating as the player sees it, rounded; `change` is how much the last attempt moved
/// it, when there was one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PuzzleRatingModel {
    pub rating: i32,
    pub deviation: i32,
    /// Too few attempts for the rating to be sure, shown with a "?".
    pub provisional: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change: Option<i32>,
}

/// What a puzzle plays as when it rates the player: its own rating and deviation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PuzzleStrength {
    pub rating: i32,
    pub rating_deviation: i32,
}
