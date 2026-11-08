use super::movement::{DIAGONAL, STRAIGHT, slide};
use crate::{Move, Position, Square};

/// Any distance in a straight line or along a diagonal; it attacks as a rook and a bishop.
pub(super) fn moves(position: &Position, from: Square, moves: &mut Vec<Move>) {
    slide(position, from, &STRAIGHT, moves);
    slide(position, from, &DIAGONAL, moves);
}
