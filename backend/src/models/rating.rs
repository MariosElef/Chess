use uuid::Uuid;

/// A rating after one result and the rating before it, keyed by the result that moved it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RatingChangeRow {
    pub source_id: Uuid,
    pub rating: f64,
    pub before: f64,
}

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
