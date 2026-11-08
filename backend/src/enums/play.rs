use chess_domain::{Outcome, Side};
use serde::{Deserialize, Serialize};

/// A side, as it travels the wire and sits in `play.games`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    White,
    Black,
}

impl Color {
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
