//! How a game stands and how it ended.

use crate::Side;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameStatus {
    Ongoing,
    Checkmate { winner: Side },
    Stalemate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    Checkmate { winner: Side },
    Resignation { winner: Side },
    Timeout { winner: Side },
    Stalemate,
    DrawAgreement,
    InsufficientMaterial,
    Repetition,
    FiftyMoves,
}

impl Outcome {
    /// `None` for every kind of draw.
    pub fn winner(self) -> Option<Side> {
        match self {
            Self::Checkmate { winner }
            | Self::Resignation { winner }
            | Self::Timeout { winner } => Some(winner),
            Self::Stalemate
            | Self::DrawAgreement
            | Self::InsufficientMaterial
            | Self::Repetition
            | Self::FiftyMoves => None,
        }
    }
}
