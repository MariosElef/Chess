//! What each side has on the board: captured pieces, the material count, and whether mate is still possible.
use crate::{Board, Piece, PieceKind, Side, Square};

/// The pieces a side can lose, most valuable first, as a captured-pieces row shows them.
const CAPTURABLE: [PieceKind; 5] = [
    PieceKind::Queen,
    PieceKind::Rook,
    PieceKind::Bishop,
    PieceKind::Knight,
    PieceKind::Pawn,
];

impl Board {
    /// The pieces of `side` that are no longer on the board, most valuable first.
    pub fn captured(&self, side: Side) -> Vec<PieceKind> {
        CAPTURABLE
            .iter()
            .flat_map(|&kind| {
                let missing = starting_count(kind).saturating_sub(self.count(side, kind));
                std::iter::repeat_n(kind, missing)
            })
            .collect()
    }

    /// White's material minus Black's, in pawns: +3 means White is a knight or bishop up.
    pub fn material_balance(&self) -> i32 {
        self.material(Side::White) - self.material(Side::Black)
    }

    /// Neither side can ever mate: only kings, one knight or bishop, or bishops on one colour.
    pub fn has_insufficient_material(&self) -> bool {
        let pieces = self.non_king_pieces();
        if pieces
            .iter()
            .any(|(_, piece)| can_force_mate_alone(piece.kind))
        {
            return false;
        }
        pieces.len() <= 1 || only_bishops_on_one_colour(&pieces)
    }

    fn count(&self, side: Side, kind: PieceKind) -> usize {
        Square::all()
            .filter(|&square| self.piece_at(square) == Some(Piece::new(side, kind)))
            .count()
    }

    fn material(&self, side: Side) -> i32 {
        CAPTURABLE
            .iter()
            .map(|&kind| {
                value(kind) * i32::try_from(self.count(side, kind)).expect("at most 64 pieces")
            })
            .sum()
    }

    fn non_king_pieces(&self) -> Vec<(Square, Piece)> {
        Square::all()
            .filter_map(|square| self.piece_at(square).map(|piece| (square, piece)))
            .filter(|(_, piece)| piece.kind != PieceKind::King)
            .collect()
    }
}

/// The usual piece values in pawns.
pub fn value(kind: PieceKind) -> i32 {
    match kind {
        PieceKind::Queen => 9,
        PieceKind::Rook => 5,
        PieceKind::Bishop | PieceKind::Knight => 3,
        PieceKind::Pawn => 1,
        PieceKind::King => 0,
    }
}

fn starting_count(kind: PieceKind) -> usize {
    match kind {
        PieceKind::King | PieceKind::Queen => 1,
        PieceKind::Rook | PieceKind::Bishop | PieceKind::Knight => 2,
        PieceKind::Pawn => 8,
    }
}

/// A pawn can promote, and a rook or queen mates with its king's help.
fn can_force_mate_alone(kind: PieceKind) -> bool {
    match kind {
        PieceKind::Pawn | PieceKind::Rook | PieceKind::Queen => true,
        PieceKind::King | PieceKind::Bishop | PieceKind::Knight => false,
    }
}

fn only_bishops_on_one_colour(pieces: &[(Square, Piece)]) -> bool {
    let all_bishops = pieces
        .iter()
        .all(|(_, piece)| piece.kind == PieceKind::Bishop);
    let first_colour = pieces.first().map(|(square, _)| square.is_light());
    all_bishops
        && pieces
            .iter()
            .all(|(square, _)| Some(square.is_light()) == first_colour)
}

#[cfg(test)]
mod tests {
    use crate::{Board, PieceKind, Position, Side};

    fn board(fen: &str) -> Board {
        *fen.parse::<Position>().expect("a valid FEN").board()
    }

    #[test]
    fn the_start_has_nothing_captured_and_even_material() {
        let start = Board::starting();
        assert!(start.captured(Side::White).is_empty());
        assert_eq!(start.material_balance(), 0);
    }

    #[test]
    fn captured_pieces_are_listed_most_valuable_first() {
        // Black has lost the queen, a knight and two pawns.
        let position = board("r1b1kb1r/pppp2pp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        assert_eq!(
            position.captured(Side::Black),
            [
                PieceKind::Queen,
                PieceKind::Knight,
                PieceKind::Knight,
                PieceKind::Pawn,
                PieceKind::Pawn
            ]
        );
        assert_eq!(position.material_balance(), 9 + 3 + 3 + 2);
    }

    #[test]
    fn lone_kings_and_single_minors_cannot_mate() {
        assert!(board("8/8/8/4k3/8/8/8/4K3 w - - 0 1").has_insufficient_material());
        assert!(board("8/8/8/4k3/8/8/8/2B1K3 w - - 0 1").has_insufficient_material());
        assert!(board("8/8/8/4k3/8/8/8/1N2K3 w - - 0 1").has_insufficient_material());
    }

    #[test]
    fn bishops_on_one_colour_cannot_mate() {
        // c1 and f8 are both dark squares.
        assert!(board("5b2/8/8/4k3/8/8/8/2B1K3 w - - 0 1").has_insufficient_material());
        // c1 is dark, c8 is light.
        assert!(!board("2b5/8/8/4k3/8/8/8/2B1K3 w - - 0 1").has_insufficient_material());
    }

    #[test]
    fn a_pawn_rook_or_two_knights_keep_the_game_going() {
        assert!(!board("8/8/8/4k3/8/8/4P3/4K3 w - - 0 1").has_insufficient_material());
        assert!(!board("8/8/8/4k3/8/8/8/R3K3 w - - 0 1").has_insufficient_material());
        assert!(!board("8/8/8/4k3/8/8/8/1N1NK3 w - - 0 1").has_insufficient_material());
    }
}
