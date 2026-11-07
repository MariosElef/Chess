use chess_domain::Side;

use super::PieceImage;

pub const WHITE: PieceImage = PieceImage {
    path: "pieces/wR.svg",
    bytes: include_bytes!("../../../../assets/pieces/wR.svg"),
};

pub const BLACK: PieceImage = PieceImage {
    path: "pieces/bR.svg",
    bytes: include_bytes!("../../../../assets/pieces/bR.svg"),
};

pub fn image(side: Side) -> PieceImage {
    match side {
        Side::White => WHITE,
        Side::Black => BLACK,
    }
}
