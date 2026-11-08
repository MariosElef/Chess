use super::movement::{DIAGONAL, line_attacks, slide};
use crate::{Board, Move, PieceKind, Position, Side, Square};

/// Any distance along a diagonal.
pub(super) fn moves(position: &Position, from: Square, moves: &mut Vec<Move>) {
    slide(position, from, &DIAGONAL, moves);
}

/// A bishop, or a queen moving like one, attacks along diagonals.
pub(super) fn attacks(board: &Board, square: Square, by: Side) -> bool {
    line_attacks(board, square, by, &DIAGONAL, PieceKind::Bishop)
}
