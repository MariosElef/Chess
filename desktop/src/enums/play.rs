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
            Self::Repetition => "threefold repetition",
            Self::FiftyMove => "the fifty-move rule",
        }
    }
}

/// Who the game is against: someone at this computer, someone online, or a bot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PlayMode {
    #[default]
    Online,
    Bots,
    Local,
}

impl PlayMode {
    /// In the order the tabs show them, the first being where Play opens.
    pub const ALL: [Self; 3] = [Self::Online, Self::Bots, Self::Local];

    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "Local",
            Self::Online => "Online",
            Self::Bots => "Bots",
        }
    }
}

/// The colour the player asks for against a bot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColourChoice {
    White,
    #[default]
    Random,
    Black,
}

impl ColourChoice {
    pub const ALL: [Self; 3] = [Self::White, Self::Random, Self::Black];

    pub fn label(self) -> &'static str {
        match self {
            Self::White => "White",
            Self::Random => "Random",
            Self::Black => "Black",
        }
    }
}
