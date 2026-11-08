use chess_domain::{Move, Position, Square};

/// What a page shows on its board: the position, how it was reached, whether moves may be made
/// on it, and the engine's best move if the page has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShownPosition {
    pub position: Position,
    pub last_move: Option<Move>,
    /// Moves and premoves are taken; when not, the board can only be looked at and drawn on.
    pub can_move: bool,
    pub engine_arrow: Option<(Square, Square)>,
}
