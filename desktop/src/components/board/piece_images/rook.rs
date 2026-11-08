use chess_domain::Side;

use super::PieceImage;
use crate::enums::settings::PieceSet;

pub const CLASSIC_WHITE: PieceImage = PieceImage {
    path: "pieces/classic/wR.svg",
    bytes: include_bytes!("../../../../assets/pieces/classic/wR.svg"),
};

pub const CLASSIC_BLACK: PieceImage = PieceImage {
    path: "pieces/classic/bR.svg",
    bytes: include_bytes!("../../../../assets/pieces/classic/bR.svg"),
};

pub const MODERN_WHITE: PieceImage = PieceImage {
    path: "pieces/modern/wR.png",
    bytes: include_bytes!("../../../../assets/pieces/modern/wR.png"),
};

pub const MODERN_BLACK: PieceImage = PieceImage {
    path: "pieces/modern/bR.png",
    bytes: include_bytes!("../../../../assets/pieces/modern/bR.png"),
};

pub fn image(side: Side, set: PieceSet) -> PieceImage {
    match (set, side) {
        (PieceSet::Classic, Side::White) => CLASSIC_WHITE,
        (PieceSet::Classic, Side::Black) => CLASSIC_BLACK,
        (PieceSet::Modern, Side::White) => MODERN_WHITE,
        (PieceSet::Modern, Side::Black) => MODERN_BLACK,
    }
}
