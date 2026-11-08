use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::rating::PuzzleRatingModel;

/// How a puzzle went: solved cleanly or not, and whether it counts for the rating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct CreateAttemptModel {
    pub solved: bool,
    pub rated: bool,
    pub duration_ms: i32,
}

/// The player's puzzle rating after a rated attempt; none for a guest or an unrated attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GetAttemptModel {
    pub rating: Option<PuzzleRatingModel>,
}

/// An attempt to store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAttempt {
    pub id: Uuid,
    pub user_id: Uuid,
    pub puzzle_id: String,
    pub solved: bool,
    pub duration_ms: i32,
    pub rated: bool,
}
