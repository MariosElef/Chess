//! How a puzzle is going, and what one move did to it.

/// Where the player stands in a puzzle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PuzzleState {
    /// The player's move.
    Solving,
    /// The opponent's scripted answer is about to be played.
    Answering,
    Solved,
}

/// What the player's move was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PuzzleVerdict {
    /// The solution's move: the opponent answers next.
    Correct,
    /// The last move of the solution, or any checkmate.
    Solved,
    /// Not the move: the board stays as it was and the player tries again.
    Wrong,
}

/// How long a Puzzle Rush lasts, as on chess.com.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RushKind {
    ThreeMinutes,
    FiveMinutes,
    /// No clock: only three mistakes end it.
    Survival,
}

impl RushKind {
    pub const ALL: [Self; 3] = [Self::ThreeMinutes, Self::FiveMinutes, Self::Survival];

    pub fn label(self) -> &'static str {
        match self {
            Self::ThreeMinutes => "3 minutes",
            Self::FiveMinutes => "5 minutes",
            Self::Survival => "Survival",
        }
    }

    /// As the server names it.
    pub fn key(self) -> &'static str {
        match self {
            Self::ThreeMinutes => "three_minutes",
            Self::FiveMinutes => "five_minutes",
            Self::Survival => "survival",
        }
    }

    /// The clock, or `None` for survival.
    pub fn limit(self) -> Option<std::time::Duration> {
        match self {
            Self::ThreeMinutes => Some(std::time::Duration::from_secs(3 * 60)),
            Self::FiveMinutes => Some(std::time::Duration::from_secs(5 * 60)),
            Self::Survival => None,
        }
    }
}
