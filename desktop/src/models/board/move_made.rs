use chess_domain::Move;

/// The side to move made a move on the board; the page decides what playing it means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveMade(pub Move);
