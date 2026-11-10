use chess_domain::Side;

use crate::i18n::tr;

/// How a side is written for people: `White`, `Black`.
pub fn side_name(side: Side) -> String {
    match side {
        Side::White => tr!("board.side.white"),
        Side::Black => tr!("board.side.black"),
    }
}
