use serde::Deserialize;

/// The player's puzzle rating; `change` is what the last attempt did to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct PuzzleRating {
    pub rating: i32,
    pub deviation: i32,
    /// Too few attempts to be sure: shown with a "?".
    pub provisional: bool,
    #[serde(default)]
    pub change: Option<i32>,
}
