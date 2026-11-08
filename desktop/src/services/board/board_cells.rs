//! Where squares are drawn: each square's cell, its column from the left and row from the top,
//! for White at the bottom or, flipped, Black.
use chess_domain::Square;

use super::arrow_shape::Cell;

pub fn cell_of(square: Square, flipped: bool) -> Cell {
    if flipped {
        (7 - square.file(), square.rank())
    } else {
        (square.file(), 7 - square.rank())
    }
}

/// The square drawn in a cell; `None` off the board.
pub fn square_in_cell((column, row): Cell, flipped: bool) -> Option<Square> {
    if column > 7 || row > 7 {
        return None;
    }
    if flipped {
        Square::new(7 - column, row)
    } else {
        Square::new(column, 7 - row)
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::Square;

    use super::{cell_of, square_in_cell};

    #[test]
    fn every_square_has_one_cell_either_way_up() {
        for flipped in [false, true] {
            for square in Square::all() {
                assert_eq!(
                    square_in_cell(cell_of(square, flipped), flipped),
                    Some(square)
                );
            }
        }
        let a1: Square = "a1".parse().expect("a square");
        assert_eq!(cell_of(a1, false), (0, 7), "bottom left for White");
        assert_eq!(cell_of(a1, true), (7, 0), "top right for Black");
        assert_eq!(square_in_cell((8, 0), false), None);
    }
}
