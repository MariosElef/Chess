use chess_domain::Side;

use super::PieceImage;
use crate::enums::settings::PieceSet;

pub const CLASSIC_WHITE: PieceImage = PieceImage {
    path: "pieces/classic/wB.svg",
    bytes: include_bytes!("../../../../assets/pieces/classic/wB.svg"),
};

pub const CLASSIC_BLACK: PieceImage = PieceImage {
    path: "pieces/classic/bB.svg",
    bytes: include_bytes!("../../../../assets/pieces/classic/bB.svg"),
};

pub const MODERN_WHITE: PieceImage = PieceImage {
    path: "pieces/modern/wB.png",
    bytes: include_bytes!("../../../../assets/pieces/modern/wB.png"),
};

pub const MODERN_BLACK: PieceImage = PieceImage {
    path: "pieces/modern/bB.png",
    bytes: include_bytes!("../../../../assets/pieces/modern/bB.png"),
};

pub fn image(side: Side, set: PieceSet) -> PieceImage {
    match (set, side) {
        (PieceSet::Classic, Side::White) => CLASSIC_WHITE,
        (PieceSet::Classic, Side::Black) => CLASSIC_BLACK,
        (PieceSet::Modern, Side::White) => MODERN_WHITE,
        (PieceSet::Modern, Side::Black) => MODERN_BLACK,
    }
}
