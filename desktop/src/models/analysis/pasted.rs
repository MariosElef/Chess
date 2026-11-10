use chess_domain::{Game, Position};

use crate::i18n::tr;

/// What was pasted into the analysis board's field: one position, or a whole game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pasted {
    /// A FEN.
    Position(Position),
    /// A PGN, with every move.
    Game(Game),
}

/// Why pasted text is neither: what it looks like, and what the parser said about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PastedProblem {
    /// It looks like a PGN, but a move in it is wrong.
    NotPgn(String),
    /// Not a PGN either: the FEN parser's complaint.
    NotFen(String),
}

impl PastedProblem {
    /// The problem in the language now shown; the parser's own detail stays as it came.
    pub fn message(&self) -> String {
        match self {
            Self::NotPgn(detail) => tr!("analysis.load.not_pgn", detail = detail),
            Self::NotFen(detail) => tr!("analysis.load.not_fen", detail = detail),
        }
    }
}
