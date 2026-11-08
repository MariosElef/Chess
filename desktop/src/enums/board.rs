//! What a click on the board means, and what a square is lit up for.

use chess_domain::{Move, Square};

/// What a click on the board does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickOutcome {
    /// Pick up a piece, switch to another, or let go (`None`).
    Select(Option<Square>),
    /// The selected piece moves to the clicked square.
    Play(Move),
    /// A pawn reaches the last rank: the player picks what it becomes first.
    Promote { from: Square, to: Square },
}

/// Whether a click on the board is the side to move making its move, or the other side
/// queueing a premove.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClickMode {
    Move,
    Premove,
}

/// What a square is lit up for; when several apply, the most urgent wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SquareHighlight {
    None,
    LastMove,
    Marked,
    Premove,
    Selected,
    Check,
}
