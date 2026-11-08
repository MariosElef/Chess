use chess_domain::Side;

use super::PieceImage;

pub const WHITE: PieceImage = PieceImage {
    path: "pieces/wK.svg",
    bytes: include_bytes!("../../../../assets/pieces/wK.svg"),
};

pub const BLACK: PieceImage = PieceImage {
    path: "pieces/bK.svg",
    bytes: include_bytes!("../../../../assets/pieces/bK.svg"),
};

pub fn image(side: Side) -> PieceImage {
    match side {
        Side::White => WHITE,
        Side::Black => BLACK,
    }
}
