use chess_domain::{Move, Position, Square};

/// What a page shows on its board: the position, how it was reached, whether moves may be made
/// on it, and the engine's best move if the page has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShownPosition {
    pub position: Position,
    pub last_move: Option<Move>,
    /// Moves and premoves are taken; when not, the board can only be looked at and drawn on.
    pub can_move: bool,
    /// The side not to move may queue premoves: a game between two players. A puzzle has one
    /// player, so its board takes only the side to move's moves.
    pub premoves: bool,
    pub engine_arrow: Option<(Square, Square)>,
}
