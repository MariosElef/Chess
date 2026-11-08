use chess_domain::Side;

/// How a side is written for people: `White`, `Black`.
pub fn side_name(side: Side) -> &'static str {
    match side {
        Side::White => "White",
        Side::Black => "Black",
    }
}
