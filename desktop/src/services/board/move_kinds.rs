//! What kind of move a move is, from the position it was played in.
use chess_domain::{Move, PieceKind, Position};

/// The king moving two files is a castle.
pub fn is_castle(before: &Position, mv: Move) -> bool {
    moved_kind(before, mv) == Some(PieceKind::King) && mv.from.file().abs_diff(mv.to.file()) == 2
}

/// Something stood on the target, or a pawn went sideways onto an empty square: en passant.
pub fn is_capture(before: &Position, mv: Move) -> bool {
    before.board().piece_at(mv.to).is_some()
        || (moved_kind(before, mv) == Some(PieceKind::Pawn) && mv.from.file() != mv.to.file())
}

fn moved_kind(before: &Position, mv: Move) -> Option<PieceKind> {
    before.board().piece_at(mv.from).map(|piece| piece.kind)
}
