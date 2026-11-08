use super::movement::{step_attacks, steps};
use crate::{Board, Move, PieceKind, Position, Side, Square};

pub(crate) const JUMPS: [(i8, i8); 8] = [
    (1, 2),
    (2, 1),
    (2, -1),
    (1, -2),
    (-1, -2),
    (-2, -1),
    (-2, 1),
    (-1, 2),
];

/// An L-shaped jump, over anything in between.
pub(super) fn moves(position: &Position, from: Square, moves: &mut Vec<Move>) {
    steps(position, from, &JUMPS, moves);
}

pub(super) fn attacks(board: &Board, square: Square, by: Side) -> bool {
    step_attacks(board, square, by, &JUMPS, PieceKind::Knight)
}
