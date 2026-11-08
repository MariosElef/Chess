use chess_domain::Side;

use super::PieceImage;
use crate::enums::settings::PieceSet;

pub const CLASSIC_WHITE: PieceImage = PieceImage {
    path: "pieces/classic/wN.svg",
    bytes: include_bytes!("../../../../assets/pieces/classic/wN.svg"),
};

pub const CLASSIC_BLACK: PieceImage = PieceImage {
    path: "pieces/classic/bN.svg",
    bytes: include_bytes!("../../../../assets/pieces/classic/bN.svg"),
};

pub const MODERN_WHITE: PieceImage = PieceImage {
    path: "pieces/modern/wN.png",
    bytes: include_bytes!("../../../../assets/pieces/modern/wN.png"),
};

pub const MODERN_BLACK: PieceImage = PieceImage {
    path: "pieces/modern/bN.png",
    bytes: include_bytes!("../../../../assets/pieces/modern/bN.png"),
};

pub const STAUNTY_WHITE: PieceImage = PieceImage {
    path: "pieces/staunty/wN.svg",
    bytes: include_bytes!("../../../../assets/pieces/staunty/wN.svg"),
};

pub const STAUNTY_BLACK: PieceImage = PieceImage {
    path: "pieces/staunty/bN.svg",
    bytes: include_bytes!("../../../../assets/pieces/staunty/bN.svg"),
};

pub const CALIFORNIA_WHITE: PieceImage = PieceImage {
    path: "pieces/california/wN.svg",
    bytes: include_bytes!("../../../../assets/pieces/california/wN.svg"),
};

pub const CALIFORNIA_BLACK: PieceImage = PieceImage {
    path: "pieces/california/bN.svg",
    bytes: include_bytes!("../../../../assets/pieces/california/bN.svg"),
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
