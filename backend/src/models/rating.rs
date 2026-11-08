use uuid::Uuid;

/// One player's rating in one category, for showing.
#[derive(Debug, Clone, PartialEq)]
pub struct UserRatingRow {
    pub user_id: Uuid,
    pub category_id: String,
    pub rating: f64,
}

/// A stored Glicko-2 rating.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RatingRow {
    pub rating: f64,
    pub deviation: f64,
    pub volatility: f64,
}

/// What moved a rating, for its history: a game, a puzzle attempt or a puzzle battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RatingSource {
    Game(Uuid),
    PuzzleAttempt(Uuid),
    Battle(Uuid),
}
