use chess_domain::Side;

use super::PieceImage;

pub const WHITE: PieceImage = PieceImage {
    path: "pieces/wB.svg",
    bytes: include_bytes!("../../../../assets/pieces/wB.svg"),
};

pub const BLACK: PieceImage = PieceImage {
    path: "pieces/bB.svg",
    bytes: include_bytes!("../../../../assets/pieces/bB.svg"),
};

pub fn image(side: Side) -> PieceImage {
    match side {
        Side::White => WHITE,
        Side::Black => BLACK,
    }
}
