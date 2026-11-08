use crate::{Piece, PieceKind, Side, Square};

const BACK_RANK: [PieceKind; 8] = [
    PieceKind::Rook,
    PieceKind::Knight,
    PieceKind::Bishop,
    PieceKind::Queen,
    PieceKind::King,
    PieceKind::Bishop,
    PieceKind::Knight,
    PieceKind::Rook,
];

/// Which piece stands on each square.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Board {
    squares: [Option<Piece>; 64],
}

impl Board {
    pub fn empty() -> Self {
        Self {
            squares: [None; 64],
        }
    }

    pub fn starting() -> Self {
        let mut board = Self::empty();
        for (file, kind) in (0..8).zip(BACK_RANK) {
            for (side, back, pawns) in [(Side::White, 0, 1), (Side::Black, 7, 6)] {
                board.place(file, back, Piece::new(side, kind));
                board.place(file, pawns, Piece::new(side, PieceKind::Pawn));
            }
        }
        board
    }

    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        self.squares[square.index()]
    }

    pub fn king_of(&self, side: Side) -> Option<Square> {
        Square::all()
            .find(|&square| self.piece_at(square) == Some(Piece::new(side, PieceKind::King)))
    }

    pub(crate) fn set(&mut self, square: Square, piece: Option<Piece>) {
        self.squares[square.index()] = piece;
    }

    fn place(&mut self, file: u8, rank: u8, piece: Piece) {
        let square = Square::new(file, rank).expect("the starting layout stays on the board");
        self.squares[square.index()] = Some(piece);
    }
}

#[cfg(test)]
mod tests {
    use super::Board;
    use crate::{Piece, PieceKind, Side, Square};

    fn at(file: u8, rank: u8) -> Square {
        Square::new(file, rank).expect("on the board")
    }

    #[test]
    fn starting_has_32_pieces() {
        let board = Board::starting();
        let count = Square::all()
            .filter(|&s| board.piece_at(s).is_some())
            .count();
        assert_eq!(count, 32);
    }

    #[test]
    fn kings_and_queens_start_on_their_files() {
        let board = Board::starting();
        assert_eq!(
            board.piece_at(at(4, 0)),
            Some(Piece::new(Side::White, PieceKind::King))
        );
        assert_eq!(
            board.piece_at(at(3, 0)),
            Some(Piece::new(Side::White, PieceKind::Queen))
        );
        assert_eq!(
            board.piece_at(at(4, 7)),
            Some(Piece::new(Side::Black, PieceKind::King))
        );
        assert_eq!(
            board.piece_at(at(3, 7)),
            Some(Piece::new(Side::Black, PieceKind::Queen))
        );
    }

    #[test]
    fn pawns_fill_the_second_and_seventh_ranks() {
        let board = Board::starting();
        for file in 0..8 {
            assert_eq!(
                board.piece_at(at(file, 1)),
                Some(Piece::new(Side::White, PieceKind::Pawn))
            );
            assert_eq!(
                board.piece_at(at(file, 6)),
                Some(Piece::new(Side::Black, PieceKind::Pawn))
            );
        }
    }

    #[test]
    fn the_middle_starts_empty() {
        let board = Board::starting();
        assert!((2..6).all(|rank| (0..8).all(|file| board.piece_at(at(file, rank)).is_none())));
    }
}
