use chess_domain::{Game, Position};

/// What was pasted into the analysis board's field: one position, or a whole game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pasted {
    /// A FEN.
    Position(Position),
    /// A PGN, with every move.
    Game(Game),
}
