//! How an online game stands, as the server says it.
use chess_domain::Side;
use serde::Deserialize;

/// A side, as it travels the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    White,
    Black,
}

impl From<Color> for Side {
    fn from(color: Color) -> Self {
        match color {
            Color::White => Self::White,
            Color::Black => Self::Black,
        }
    }
}

/// Whether an online game is still being played.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveStatus {
    Ongoing,
    Finished,
    /// Called off before both sides had moved: no result, never rated.
    Aborted,
}

/// How an online game ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
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
    /// The words after "by": `checkmate`, `agreement`, `the fifty-move rule`…
    pub fn by(self) -> &'static str {
        match self {
            Self::Checkmate => "checkmate",
            Self::Resignation => "resignation",
            Self::Timeout => "time",
            Self::Stalemate => "stalemate",
            Self::DrawAgreement => "agreement",
            Self::InsufficientMaterial => "insufficient material",
            Self::Repetition => "repetition",
            Self::FiftyMove => "the fifty-move rule",
        }
    }
}

/// Who the game is against: someone at this computer, or someone online.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlayMode {
    #[default]
    Local,
    Online,
}

impl PlayMode {
    pub const ALL: [Self; 2] = [Self::Local, Self::Online];

    pub fn index(self) -> usize {
        match self {
            Self::Local => 0,
            Self::Online => 1,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "On this computer",
            Self::Online => "Online",
        }
    }
}
