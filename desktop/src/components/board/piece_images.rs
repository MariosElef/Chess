mod bishop;
mod king;
mod knight;
mod pawn;
mod queen;
mod rook;

use chess_domain::{Piece, PieceKind};

use crate::enums::settings::PieceSet;

/// One piece's picture (an SVG or a PNG), compiled into the binary and served to `img()`
/// under `path`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PieceImage {
    pub path: &'static str,
    pub bytes: &'static [u8],
}

/// Every picture of every set, for the asset source to serve.
pub const ALL: [PieceImage; 24] = [
    king::CLASSIC_WHITE,
    king::CLASSIC_BLACK,
    king::MODERN_WHITE,
    king::MODERN_BLACK,
    queen::CLASSIC_WHITE,
    queen::CLASSIC_BLACK,
    queen::MODERN_WHITE,
    queen::MODERN_BLACK,
    rook::CLASSIC_WHITE,
    rook::CLASSIC_BLACK,
    rook::MODERN_WHITE,
    rook::MODERN_BLACK,
    bishop::CLASSIC_WHITE,
    bishop::CLASSIC_BLACK,
    bishop::MODERN_WHITE,
    bishop::MODERN_BLACK,
    knight::CLASSIC_WHITE,
    knight::CLASSIC_BLACK,
    knight::MODERN_WHITE,
    knight::MODERN_BLACK,
    pawn::CLASSIC_WHITE,
    pawn::CLASSIC_BLACK,
    pawn::MODERN_WHITE,
    pawn::MODERN_BLACK,
];

pub fn piece_image(piece: Piece, set: PieceSet) -> PieceImage {
    match piece.kind {
        PieceKind::King => king::image(piece.side, set),
        PieceKind::Queen => queen::image(piece.side, set),
        PieceKind::Rook => rook::image(piece.side, set),
        PieceKind::Bishop => bishop::image(piece.side, set),
        PieceKind::Knight => knight::image(piece.side, set),
        PieceKind::Pawn => pawn::image(piece.side, set),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use chess_domain::{Piece, PieceKind, Side};

    use super::{ALL, piece_image};
    use crate::enums::settings::PieceSet;

    #[test]
    fn every_picture_has_its_own_path_and_is_served() {
        let paths: HashSet<&str> = ALL.iter().map(|image| image.path).collect();
        assert_eq!(paths.len(), ALL.len());
        let kinds = [
            PieceKind::King,
            PieceKind::Queen,
            PieceKind::Rook,
            PieceKind::Bishop,
            PieceKind::Knight,
            PieceKind::Pawn,
        ];
        for set in PieceSet::ALL {
            for side in [Side::White, Side::Black] {
                for kind in kinds {
                    let image = piece_image(Piece::new(side, kind), set);
                    assert!(paths.contains(image.path), "{} is served", image.path);
                    assert!(!image.bytes.is_empty());
                }
            }
        }
    }

    #[test]
    fn each_set_draws_from_its_own_folder() {
        let knight = Piece::new(Side::Black, PieceKind::Knight);
        assert_eq!(
            piece_image(knight, PieceSet::Classic).path,
            "pieces/classic/bN.svg"
        );
        assert_eq!(
            piece_image(knight, PieceSet::Modern).path,
            "pieces/modern/bN.png"
        );
    }
}
