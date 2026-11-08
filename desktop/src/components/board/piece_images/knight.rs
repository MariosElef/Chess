use chess_domain::Side;

use super::PieceImage;

pub const WHITE: PieceImage = PieceImage {
    path: "pieces/wN.svg",
    bytes: include_bytes!("../../../../assets/pieces/wN.svg"),
};

pub const BLACK: PieceImage = PieceImage {
    path: "pieces/bN.svg",
    bytes: include_bytes!("../../../../assets/pieces/bN.svg"),
};

pub fn image(side: Side) -> PieceImage {
    match side {
        Side::White => WHITE,
        Side::Black => BLACK,
    }
}
