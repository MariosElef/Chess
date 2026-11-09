use serde::Deserialize;

/// A player's place on a leaderboard.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RankedPlayer {
    pub place: i64,
    pub id: String,
    pub username: String,
    /// Their picture's hash; none while they show their initial.
    #[serde(default)]
    pub avatar: Option<String>,
    pub rating: i32,
    /// Too few results for the rating to be sure of yet: it shows with a "?".
    pub provisional: bool,
    /// Rated games, puzzles or battles played on the board.
    pub played: i32,
}

/// A page of a leaderboard, the player's own place, and whether more come after it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RankingPage {
    pub rows: Vec<RankedPlayer>,
    pub me: Option<RankedPlayer>,
    pub more: bool,
}
