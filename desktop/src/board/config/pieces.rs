mod bishop;
mod king;
mod knight;
mod pawn;
mod queen;
mod rook;

use chess_domain::{Piece, PieceKind};

/// One piece's SVG, compiled into the binary and served to `img()` under `path`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PieceImage {
    pub path: &'static str,
    pub bytes: &'static [u8],
}

pub const ALL: [PieceImage; 12] = [
    king::WHITE,
    king::BLACK,
    queen::WHITE,
    queen::BLACK,
    rook::WHITE,
    rook::BLACK,
    bishop::WHITE,
    bishop::BLACK,
    knight::WHITE,
    knight::BLACK,
    pawn::WHITE,
    pawn::BLACK,
];

pub fn piece_image(piece: Piece) -> PieceImage {
    match piece.kind {
        PieceKind::King => king::image(piece.side),
        PieceKind::Queen => queen::image(piece.side),
        PieceKind::Rook => rook::image(piece.side),
        PieceKind::Bishop => bishop::image(piece.side),
        PieceKind::Knight => knight::image(piece.side),
        PieceKind::Pawn => pawn::image(piece.side),
    }
}
