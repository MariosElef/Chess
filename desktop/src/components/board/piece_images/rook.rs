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

pub const STAUNTY_WHITE: PieceImage = PieceImage {
    path: "pieces/staunty/wR.svg",
    bytes: include_bytes!("../../../../assets/pieces/staunty/wR.svg"),
};

pub const STAUNTY_BLACK: PieceImage = PieceImage {
    path: "pieces/staunty/bR.svg",
    bytes: include_bytes!("../../../../assets/pieces/staunty/bR.svg"),
};

pub const CALIFORNIA_WHITE: PieceImage = PieceImage {
    path: "pieces/california/wR.svg",
    bytes: include_bytes!("../../../../assets/pieces/california/wR.svg"),
};

pub const CALIFORNIA_BLACK: PieceImage = PieceImage {
    path: "pieces/california/bR.svg",
    bytes: include_bytes!("../../../../assets/pieces/california/bR.svg"),
};

pub fn image(side: Side, set: PieceSet) -> PieceImage {
    match (set, side) {
        (PieceSet::Classic, Side::White) => CLASSIC_WHITE,
        (PieceSet::Classic, Side::Black) => CLASSIC_BLACK,
        (PieceSet::Modern, Side::White) => MODERN_WHITE,
        (PieceSet::Modern, Side::Black) => MODERN_BLACK,
        (PieceSet::Staunty, Side::White) => STAUNTY_WHITE,
        (PieceSet::Staunty, Side::Black) => STAUNTY_BLACK,
        (PieceSet::California, Side::White) => CALIFORNIA_WHITE,
        (PieceSet::California, Side::Black) => CALIFORNIA_BLACK,
    }
}
