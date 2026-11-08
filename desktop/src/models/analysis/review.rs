use chess_domain::{Move, Side, engine::Eval};

use crate::enums::analysis::MoveClass;

/// One of the engine's lines at a position: its first move and what it is worth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineLine {
    /// `None` for a finished position, which has no move.
    pub first: Option<Move>,
    /// From White's point of view.
    pub eval: Eval,
}

/// What the engine said about one position of the game: its best lines, best first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionEval {
    pub lines: Vec<EngineLine>,
}

/// One move of the game as the review sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveReview {
    pub mover: Side,
    pub class: MoveClass,
    /// The engine's best move in the position the move was played from.
    pub best: Option<Move>,
    /// How close to best, from 0 to 100.
    pub accuracy: f64,
}

/// A whole game reviewed: every move in order, and each side's accuracy.
#[derive(Debug, Clone, PartialEq)]
pub struct GameReview {
    pub moves: Vec<MoveReview>,
    /// White's, then Black's, from 0 to 100.
    pub accuracy: [f64; 2],
    /// The rating each side played like, White's then Black's; `None` with too few moves of
    /// their own to go on.
    pub ratings: [Option<u32>; 2],
}
