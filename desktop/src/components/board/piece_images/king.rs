use chess_domain::Side;

use super::PieceImage;
use crate::enums::settings::PieceSet;

pub const CLASSIC_WHITE: PieceImage = PieceImage {
    path: "pieces/classic/wK.svg",
    bytes: include_bytes!("../../../../assets/pieces/classic/wK.svg"),
};

pub const CLASSIC_BLACK: PieceImage = PieceImage {
    path: "pieces/classic/bK.svg",
    bytes: include_bytes!("../../../../assets/pieces/classic/bK.svg"),
};

pub const MODERN_WHITE: PieceImage = PieceImage {
    path: "pieces/modern/wK.png",
    bytes: include_bytes!("../../../../assets/pieces/modern/wK.png"),
};

pub const MODERN_BLACK: PieceImage = PieceImage {
    path: "pieces/modern/bK.png",
    bytes: include_bytes!("../../../../assets/pieces/modern/bK.png"),
};

pub fn image(side: Side, set: PieceSet) -> PieceImage {
    match (set, side) {
        (PieceSet::Classic, Side::White) => CLASSIC_WHITE,
        (PieceSet::Classic, Side::Black) => CLASSIC_BLACK,
        (PieceSet::Modern, Side::White) => MODERN_WHITE,
        (PieceSet::Modern, Side::Black) => MODERN_BLACK,
    }
}
