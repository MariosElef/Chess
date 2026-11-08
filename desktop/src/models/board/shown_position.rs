use chess_domain::{Move, Outcome, Position, Side, Square};

use crate::enums::analysis::MoveClass;

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
    /// The board is this player's, as in an online game: only their pieces move, now or as
    /// premoves. `None` lets either side move, as two players at one computer do.
    pub player: Option<Side>,
    pub engine_arrow: Option<(Square, Square)>,
    /// What should have been played instead of the move that led here, once reviewed.
    pub best_arrow: Option<(Square, Square)>,
    /// A reviewed move's class, drawn on the square it went to.
    pub badge: Option<(Square, MoveClass)>,
    /// How the game ended, while its final position is shown: the kings wear it.
    pub outcome: Option<Outcome>,
}

impl ShownPosition {
    /// The starting position, waiting for a game or a puzzle: nothing moves on it yet.
    pub fn waiting() -> Self {
        Self {
            position: Position::starting(),
            last_move: None,
            can_move: false,
            premoves: false,
            player: None,
            engine_arrow: None,
            best_arrow: None,
            badge: None,
            outcome: None,
        }
    }
}
