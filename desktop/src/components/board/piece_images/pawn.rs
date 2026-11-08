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

pub const STAUNTY_WHITE: PieceImage = PieceImage {
    path: "pieces/staunty/wP.svg",
    bytes: include_bytes!("../../../../assets/pieces/staunty/wP.svg"),
};

pub const STAUNTY_BLACK: PieceImage = PieceImage {
    path: "pieces/staunty/bP.svg",
    bytes: include_bytes!("../../../../assets/pieces/staunty/bP.svg"),
};

pub const CALIFORNIA_WHITE: PieceImage = PieceImage {
    path: "pieces/california/wP.svg",
    bytes: include_bytes!("../../../../assets/pieces/california/wP.svg"),
};

pub const CALIFORNIA_BLACK: PieceImage = PieceImage {
    path: "pieces/california/bP.svg",
    bytes: include_bytes!("../../../../assets/pieces/california/bP.svg"),
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
