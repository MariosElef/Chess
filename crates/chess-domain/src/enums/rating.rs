//! A game's result for the player being rated.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Score {
    Win,
    Draw,
    Loss,
}

impl Score {
    /// The points a game is worth: 1, ½ or 0.
    pub(crate) fn value(self) -> f64 {
        match self {
            Self::Win => 1.0,
            Self::Draw => 0.5,
            Self::Loss => 0.0,
        }
    }
}
