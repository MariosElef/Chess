use uuid::Uuid;

/// A stored Glicko-2 rating.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RatingRow {
    pub rating: f64,
    pub deviation: f64,
    pub volatility: f64,
}

/// What moved a rating, for its history: a game or a puzzle attempt, never both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RatingSource {
    Game(Uuid),
    PuzzleAttempt(Uuid),
}
