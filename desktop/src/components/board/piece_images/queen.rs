use chess_domain::Side;

use super::PieceImage;
use crate::enums::settings::PieceSet;

pub const CLASSIC_WHITE: PieceImage = PieceImage {
    path: "pieces/classic/wQ.svg",
    bytes: include_bytes!("../../../../assets/pieces/classic/wQ.svg"),
};

pub const CLASSIC_BLACK: PieceImage = PieceImage {
    path: "pieces/classic/bQ.svg",
    bytes: include_bytes!("../../../../assets/pieces/classic/bQ.svg"),
};

pub const MODERN_WHITE: PieceImage = PieceImage {
    path: "pieces/modern/wQ.png",
    bytes: include_bytes!("../../../../assets/pieces/modern/wQ.png"),
};

pub const MODERN_BLACK: PieceImage = PieceImage {
    path: "pieces/modern/bQ.png",
    bytes: include_bytes!("../../../../assets/pieces/modern/bQ.png"),
};

pub const STAUNTY_WHITE: PieceImage = PieceImage {
    path: "pieces/staunty/wQ.svg",
    bytes: include_bytes!("../../../../assets/pieces/staunty/wQ.svg"),
};

pub const STAUNTY_BLACK: PieceImage = PieceImage {
    path: "pieces/staunty/bQ.svg",
    bytes: include_bytes!("../../../../assets/pieces/staunty/bQ.svg"),
};

pub const CALIFORNIA_WHITE: PieceImage = PieceImage {
    path: "pieces/california/wQ.svg",
    bytes: include_bytes!("../../../../assets/pieces/california/wQ.svg"),
};

pub const CALIFORNIA_BLACK: PieceImage = PieceImage {
    path: "pieces/california/bQ.svg",
    bytes: include_bytes!("../../../../assets/pieces/california/bQ.svg"),
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
