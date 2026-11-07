use chess_domain::Side;

use super::PieceImage;

pub const WHITE: PieceImage = PieceImage {
    path: "pieces/wP.svg",
    bytes: include_bytes!("../../../../assets/pieces/wP.svg"),
};

pub const BLACK: PieceImage = PieceImage {
    path: "pieces/bP.svg",
    bytes: include_bytes!("../../../../assets/pieces/bP.svg"),
};

pub fn image(side: Side) -> PieceImage {
    match side {
        Side::White => WHITE,
        Side::Black => BLACK,
    }
}
