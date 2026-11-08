use chess_domain::Side;

use super::PieceImage;

pub const WHITE: PieceImage = PieceImage {
    path: "pieces/wQ.svg",
    bytes: include_bytes!("../../../../assets/pieces/wQ.svg"),
};

pub const BLACK: PieceImage = PieceImage {
    path: "pieces/bQ.svg",
    bytes: include_bytes!("../../../../assets/pieces/bQ.svg"),
};

pub fn image(side: Side) -> PieceImage {
    match side {
        Side::White => WHITE,
        Side::Black => BLACK,
    }
}
