use super::movement::{STRAIGHT, line_attacks, slide};
use crate::{Board, Move, PieceKind, Position, Side, Square};

/// Any distance along a rank or a file.
pub(super) fn moves(position: &Position, from: Square, moves: &mut Vec<Move>) {
    slide(position, from, &STRAIGHT, moves);
}

/// A rook, or a queen moving like one, attacks along ranks and files.
pub(super) fn attacks(board: &Board, square: Square, by: Side) -> bool {
    line_attacks(board, square, by, &STRAIGHT, PieceKind::Rook)
}
