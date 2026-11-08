//! How pieces reach squares: single steps (king, knight) and slides along lines (rook, bishop, queen).
use crate::{Board, Move, Piece, PieceKind, Position, Side, Square};

pub(crate) const STRAIGHT: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
pub(crate) const DIAGONAL: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

/// Moves to each square one offset away that is empty or holds an enemy piece.
pub(super) fn steps(
    position: &Position,
    from: Square,
    offsets: &[(i8, i8)],
    moves: &mut Vec<Move>,
) {
    for &(files, ranks) in offsets {
        if let Some(to) = from.offset(files, ranks)
            && can_land(position, to)
        {
            moves.push(Move::new(from, to));
        }
    }
}

/// Moves along each direction until the edge, a friendly piece, or a capture.
pub(super) fn slide(
    position: &Position,
    from: Square,
    directions: &[(i8, i8)],
    moves: &mut Vec<Move>,
) {
    for &(files, ranks) in directions {
        slide_line(position, from, (files, ranks), moves);
    }
}

/// Whether a `kind` piece of `by` sits one offset away from `square`.
pub(super) fn step_attacks(
    board: &Board,
    square: Square,
    by: Side,
    offsets: &[(i8, i8)],
    kind: PieceKind,
) -> bool {
    let attacker = Some(Piece::new(by, kind));
    offsets.iter().any(|&(files, ranks)| {
        square
            .offset(files, ranks)
            .is_some_and(|from| board.piece_at(from) == attacker)
    })
}

/// Whether the first piece in any direction from `square` is a `kind` or queen of `by`.
pub(super) fn line_attacks(
    board: &Board,
    square: Square,
    by: Side,
    directions: &[(i8, i8)],
    kind: PieceKind,
) -> bool {
    directions.iter().any(|&direction| {
        first_piece(board, square, direction)
            .is_some_and(|piece| attacks_along_line(piece, by, kind))
    })
}

/// A square a piece of the side to move may finish on: empty, or holding an enemy.
pub(super) fn can_land(position: &Position, square: Square) -> bool {
    position
        .board()
        .piece_at(square)
        .is_none_or(|piece| piece.side != position.side_to_move())
}

fn slide_line(position: &Position, from: Square, (files, ranks): (i8, i8), moves: &mut Vec<Move>) {
    let mut current = from;
    while let Some(to) = current.offset(files, ranks) {
        if can_land(position, to) {
            moves.push(Move::new(from, to));
        }
        if position.board().piece_at(to).is_some() {
            return;
        }
        current = to;
    }
}

fn first_piece(board: &Board, from: Square, (files, ranks): (i8, i8)) -> Option<Piece> {
    let mut current = from;
    while let Some(next) = current.offset(files, ranks) {
        if let Some(piece) = board.piece_at(next) {
            return Some(piece);
        }
        current = next;
    }
    None
}

fn attacks_along_line(piece: Piece, by: Side, kind: PieceKind) -> bool {
    piece.side == by && (piece.kind == kind || piece.kind == PieceKind::Queen)
}
