use chess_domain::Side;

use super::PieceImage;
use crate::enums::settings::PieceSet;

pub const CLASSIC_WHITE: PieceImage = PieceImage {
    path: "pieces/classic/wP.svg",
    bytes: include_bytes!("../../../../assets/pieces/classic/wP.svg"),
};

pub const CLASSIC_BLACK: PieceImage = PieceImage {
    path: "pieces/classic/bP.svg",
    bytes: include_bytes!("../../../../assets/pieces/classic/bP.svg"),
};

pub const MODERN_WHITE: PieceImage = PieceImage {
    path: "pieces/modern/wP.png",
    bytes: include_bytes!("../../../../assets/pieces/modern/wP.png"),
};

pub const MODERN_BLACK: PieceImage = PieceImage {
    path: "pieces/modern/bP.png",
    bytes: include_bytes!("../../../../assets/pieces/modern/bP.png"),
};

pub fn image(side: Side, set: PieceSet) -> PieceImage {
    match (set, side) {
        (PieceSet::Classic, Side::White) => CLASSIC_WHITE,
        (PieceSet::Classic, Side::Black) => CLASSIC_BLACK,
        (PieceSet::Modern, Side::White) => MODERN_WHITE,
        (PieceSet::Modern, Side::Black) => MODERN_BLACK,
    }
}
