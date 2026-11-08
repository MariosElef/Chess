//! A move that could not be read, or cannot be played.

use crate::{Move, ParseSquareError};

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMoveError {
    Length,
    Square(ParseSquareError),
    Promotion(char),
}

impl std::fmt::Display for ParseMoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Length => f.write_str("a move is four or five characters, like e2e4 or e7e8q"),
            Self::Square(error) => error.fmt(f),
            Self::Promotion(letter) => write!(f, "{letter:?} is not a promotion piece"),
        }
    }
}

impl std::error::Error for ParseMoveError {}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IllegalMove(pub Move);

impl std::fmt::Display for IllegalMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} is not a legal move here", self.0)
    }
}

impl std::error::Error for IllegalMove {}
