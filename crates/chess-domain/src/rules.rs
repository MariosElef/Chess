mod bishop;
mod king;
mod knight;
mod movement;
mod pawn;
mod queen;
mod rook;

use crate::{Board, Move, Piece, PieceKind, Position, Side, Square};

/// Every move the side to move's pieces can make, before checking whether it leaves their king in check.
pub(crate) fn pseudo_legal_moves(position: &Position) -> Vec<Move> {
    let mut moves = Vec::new();
    for from in Square::all() {
        if let Some(piece) = own_piece(position, from) {
            piece_moves(position, piece, from, &mut moves);
        }
    }
    moves
}

/// Whether any piece of `by` could capture on `square`.
pub(crate) fn is_attacked(board: &Board, square: Square, by: Side) -> bool {
    king::attacks(board, square, by)
        || knight::attacks(board, square, by)
        || pawn::attacks(board, square, by)
        || rook::attacks(board, square, by)
        || bishop::attacks(board, square, by)
}

fn own_piece(position: &Position, square: Square) -> Option<Piece> {
    position
        .board()
        .piece_at(square)
        .filter(|piece| piece.side == position.side_to_move())
}

fn piece_moves(position: &Position, piece: Piece, from: Square, moves: &mut Vec<Move>) {
    match piece.kind {
        PieceKind::King => king::moves(position, from, moves),
        PieceKind::Queen => queen::moves(position, from, moves),
        PieceKind::Rook => rook::moves(position, from, moves),
        PieceKind::Bishop => bishop::moves(position, from, moves),
        PieceKind::Knight => knight::moves(position, from, moves),
        PieceKind::Pawn => pawn::moves(position, from, moves),
    }
}
