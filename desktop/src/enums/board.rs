//! What a click on the board means, and what a square is lit up for.

use chess_domain::{Move, Side, Square};

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

/// Whether a board click is the side to move moving, or the other side queueing a premove.
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
    /// The last move once reviewed, in its class's colour (`0xRRGGBB`).
    Reviewed(u32),
    Marked,
    Premove,
    Selected,
    Check,
}

/// What a king shows once the game is over: the winner's crown, the loser's reason, or a draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EndBadge {
    Winner,
    Checkmated(Side),
    Resigned(Side),
    OutOfTime(Side),
    Drawn(Side),
}

impl EndBadge {
    /// The icon's asset path.
    pub fn icon_path(self) -> &'static str {
        match self {
            Self::Winner => "end/winner.png",
            Self::Checkmated(Side::White) => "end/checkmate_white.png",
            Self::Checkmated(Side::Black) => "end/checkmate_black.png",
            Self::Resigned(Side::White) => "end/resign_white.png",
            Self::Resigned(Side::Black) => "end/resign_black.png",
            Self::OutOfTime(Side::White) => "end/timeout_white.png",
            Self::OutOfTime(Side::Black) => "end/timeout_black.png",
            Self::Drawn(Side::White) => "end/draw_white.png",
            Self::Drawn(Side::Black) => "end/draw_black.png",
        }
    }
}
