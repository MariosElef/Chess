use chess_domain::{Move, Position};

/// A puzzle to solve: the position after the opponent's last move, and the moves that win.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    pub id: String,
    pub rating: u32,
    /// How many times it has been played on Lichess.
    pub plays: u32,
    pub themes: Vec<String>,
    pub position: Position,
    /// The opponent's move that led to the position, highlighted on the board.
    pub last_move: Option<Move>,
    pub solution: Vec<Move>,
}
