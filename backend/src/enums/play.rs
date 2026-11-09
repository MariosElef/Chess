use chess_domain::{Category, Outcome, Side};
use serde::{Deserialize, Serialize};

/// A side, as it travels the wire and sits in `play.games`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub const ALL: [Self; 2] = [Self::White, Self::Black];

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|color| color.key() == key)
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::White => "white",
            Self::Black => "black",
        }
    }
}

impl From<Side> for Color {
    fn from(side: Side) -> Self {
        match side {
            Side::White => Self::White,
            Side::Black => Self::Black,
        }
    }
}

impl From<Color> for Side {
    fn from(color: Color) -> Self {
        match color {
            Color::White => Self::White,
            Color::Black => Self::Black,
        }
    }
}

/// How a finished game went for one player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerResult {
    Won,
    Lost,
    Drawn,
}

/// Whether a live game is still being played.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    Ongoing,
    Finished,
    /// Called off before both sides had moved: no result, never rated.
    Aborted,
}

/// How a game ended, with the names `play.games.outcome` allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameResult {
    Checkmate,
    Resignation,
    Timeout,
    Stalemate,
    DrawAgreement,
    InsufficientMaterial,
    Repetition,
    FiftyMove,
}

impl GameResult {
    pub const ALL: [Self; 8] = [
        Self::Checkmate,
        Self::Resignation,
        Self::Timeout,
        Self::Stalemate,
        Self::DrawAgreement,
        Self::InsufficientMaterial,
        Self::Repetition,
        Self::FiftyMove,
    ];

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|result| result.key() == key)
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Checkmate => "checkmate",
            Self::Resignation => "resignation",
            Self::Timeout => "timeout",
            Self::Stalemate => "stalemate",
            Self::DrawAgreement => "draw_agreement",
            Self::InsufficientMaterial => "insufficient_material",
            Self::Repetition => "repetition",
            Self::FiftyMove => "fifty_move",
        }
    }
}

impl From<Outcome> for GameResult {
    fn from(outcome: Outcome) -> Self {
        match outcome {
            Outcome::Checkmate { .. } => Self::Checkmate,
            Outcome::Resignation { .. } => Self::Resignation,
            Outcome::Timeout { .. } => Self::Timeout,
            Outcome::Stalemate => Self::Stalemate,
            Outcome::DrawAgreement => Self::DrawAgreement,
            Outcome::InsufficientMaterial => Self::InsufficientMaterial,
            Outcome::Repetition => Self::Repetition,
            Outcome::FiftyMoves => Self::FiftyMove,
        }
    }
}

/// How a challenge was settled, as `play.challenges.status` stores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChallengeStatus {
    Accepted,
    Declined,
    Cancelled,
    Expired,
}

impl ChallengeStatus {
    pub fn key(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Declined => "declined",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
        }
    }
}

/// Puzzle ratings are kept in this category of `play.ratings`.
pub const PUZZLE_CATEGORY: &str = "puzzle";
/// The category Puzzle Battle's rating moves in.
pub const BATTLE_CATEGORY: &str = "puzzle_battle";

/// A leaderboard: one per rating category, and one of each player's average over them all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RatingBoard {
    Speed(Category),
    Puzzles,
    PuzzleBattle,
    Average,
}

impl RatingBoard {
    pub const ALL: [Self; 7] = [
        Self::Speed(Category::Bullet),
        Self::Speed(Category::Blitz),
        Self::Speed(Category::Rapid),
        Self::Speed(Category::Classical),
        Self::Puzzles,
        Self::PuzzleBattle,
        Self::Average,
    ];

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|board| board.key() == key)
    }

    /// The board's name in the address: its rating category's id, or `average`.
    pub fn key(self) -> String {
        self.category().unwrap_or_else(|| "average".to_owned())
    }

    /// The rating category the board ranks; the average board ranks them all.
    pub fn category(self) -> Option<String> {
        match self {
            Self::Speed(category) => Some(category.name().to_lowercase()),
            Self::Puzzles => Some(PUZZLE_CATEGORY.to_owned()),
            Self::PuzzleBattle => Some(BATTLE_CATEGORY.to_owned()),
            Self::Average => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RatingBoard;

    #[test]
    fn every_board_is_found_again_by_its_key() {
        for board in RatingBoard::ALL {
            assert_eq!(RatingBoard::from_key(&board.key()), Some(board));
        }
        let keys: Vec<String> = RatingBoard::ALL.into_iter().map(RatingBoard::key).collect();
        assert_eq!(
            keys,
            [
                "bullet",
                "blitz",
                "rapid",
                "classical",
                "puzzle",
                "puzzle_battle",
                "average"
            ],
            "the ids `play.rating_categories` holds, then the average"
        );
    }
}
