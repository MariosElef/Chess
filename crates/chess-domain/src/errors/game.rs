//! A game action that was refused.

use crate::IllegalMove;

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameError {
    Illegal(IllegalMove),
    Over,
    NoDrawOffer,
}

impl From<IllegalMove> for GameError {
    fn from(illegal: IllegalMove) -> Self {
        Self::Illegal(illegal)
    }
}

impl std::fmt::Display for GameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Illegal(illegal) => illegal.fmt(f),
            Self::Over => f.write_str("the game is over"),
            Self::NoDrawOffer => f.write_str("there is no draw offer to answer"),
        }
    }
}

impl std::error::Error for GameError {}
