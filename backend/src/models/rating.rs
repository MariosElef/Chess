use serde::{Deserialize, Serialize};
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

/// A player's place on a leaderboard, as the query gives it.
#[derive(Debug, Clone, PartialEq)]
pub struct LeaderRow {
    pub user_id: Uuid,
    pub username: String,
    pub avatar: Option<String>,
    pub rating: f64,
    pub deviation: f64,
    pub volatility: f64,
    pub games_played: i32,
    pub place: i64,
}

/// One row of a leaderboard: who, where, and their rating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LeaderModel {
    pub place: i64,
    pub id: Uuid,
    pub username: String,
    pub avatar: Option<String>,
    pub rating: i32,
    /// Too few results yet for the rating to be sure of: it shows with a "?".
    pub provisional: bool,
    /// Rated games, puzzles or battles played in the category.
    pub played: i32,
}

/// A page of a leaderboard, the asker's own place, and whether more come after it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LeaderboardModel {
    pub rows: Vec<LeaderModel>,
    /// The asker's place, if they are on the board; none for a guest.
    pub me: Option<LeaderModel>,
    pub more: bool,
}

/// Which page of a leaderboard: from `offset` places down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct LeaderboardQuery {
    #[serde(default)]
    pub offset: i64,
}
