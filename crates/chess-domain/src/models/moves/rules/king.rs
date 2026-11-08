use super::{
    is_attacked,
    movement::{step_attacks, steps},
};
use crate::{Board, Move, Piece, PieceKind, Position, Side, Square};

pub(crate) const STEPS: [(i8, i8); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

/// The files the king crosses, and those that must be empty, for each castle.
const KING_SIDE: Castle = Castle {
    rook_file: 7,
    king_to_file: 6,
    must_be_empty: &[5, 6],
    must_be_safe: &[5, 6],
};
const QUEEN_SIDE: Castle = Castle {
    rook_file: 0,
    king_to_file: 2,
    must_be_empty: &[1, 2, 3],
    must_be_safe: &[2, 3],
};

struct Castle {
    rook_file: u8,
    king_to_file: u8,
    must_be_empty: &'static [u8],
    must_be_safe: &'static [u8],
}

/// One square in any direction, plus castling.
pub(super) fn moves(position: &Position, from: Square, moves: &mut Vec<Move>) {
    steps(position, from, &STEPS, moves);
    if can_start_castling(position, from) {
        castle(
            position,
            from,
            &KING_SIDE,
            position.castling().king_side(position.side_to_move()),
            moves,
        );
        castle(
            position,
            from,
            &QUEEN_SIDE,
            position.castling().queen_side(position.side_to_move()),
            moves,
        );
    }
}

pub(super) fn attacks(board: &Board, square: Square, by: Side) -> bool {
    step_attacks(board, square, by, &STEPS, PieceKind::King)
}

/// The king must stand on its home square and not be in check.
fn can_start_castling(position: &Position, from: Square) -> bool {
    let side = position.side_to_move();
    from == home(4, side.back_rank()) && !is_attacked(position.board(), from, side.opponent())
}

/// The king moves two squares toward a rook that has not moved, over empty squares, never through
/// or into check.
fn castle(
    position: &Position,
    from: Square,
    castle: &Castle,
    allowed: bool,
    moves: &mut Vec<Move>,
) {
    let side = position.side_to_move();
    let rank = side.back_rank();
    let board = position.board();
    if allowed
        && rook_at_home(board, side, home(castle.rook_file, rank))
        && all_empty(board, rank, castle.must_be_empty)
        && none_attacked(board, rank, castle.must_be_safe, side.opponent())
    {
        moves.push(Move::new(from, home(castle.king_to_file, rank)));
    }
}

fn rook_at_home(board: &Board, side: Side, corner: Square) -> bool {
    board.piece_at(corner) == Some(Piece::new(side, PieceKind::Rook))
}

fn all_empty(board: &Board, rank: u8, files: &[u8]) -> bool {
    files
        .iter()
        .all(|&file| board.piece_at(home(file, rank)).is_none())
}

fn none_attacked(board: &Board, rank: u8, files: &[u8], by: Side) -> bool {
    files
        .iter()
        .all(|&file| !is_attacked(board, home(file, rank), by))
}

fn home(file: u8, rank: u8) -> Square {
    Square::new(file, rank).expect("a back-rank square")
}
