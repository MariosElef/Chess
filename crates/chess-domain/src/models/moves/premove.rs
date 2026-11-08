//! Premoves, as chess.com plays them: where a piece could ever go from its square, with no
//! regard for what is in the way or whose turn it is. Whether the move is legal is decided
//! when its turn comes, by the ordinary rules.
use crate::{
    Board, Move, Piece, PieceKind, Side, Square,
    models::moves::rules::{DIAGONAL, KING_STEPS, KNIGHT_JUMPS, STRAIGHT},
};

/// The files the king lands on when castling either way.
const CASTLE_FILES: [u8; 2] = [2, 6];

impl Board {
    /// Every square the piece on `from` could reach by its own way of moving, ignoring every
    /// other piece: a slider's full rays, a pawn's pushes and both captures, the king's castles.
    pub fn premove_targets(&self, from: Square) -> Vec<Square> {
        let Some(piece) = self.piece_at(from) else {
            return Vec::new();
        };
        let mut targets = match piece.kind {
            PieceKind::King => king_targets(from, piece.side),
            PieceKind::Queen => rays(from, &STRAIGHT).chain(rays(from, &DIAGONAL)).collect(),
            PieceKind::Rook => rays(from, &STRAIGHT).collect(),
            PieceKind::Bishop => rays(from, &DIAGONAL).collect(),
            PieceKind::Knight => steps(from, &KNIGHT_JUMPS).collect(),
            PieceKind::Pawn => pawn_targets(from, piece.side),
        };
        targets.sort_unstable();
        targets
    }

    /// The pieces as they will stand if `mv` happens, with no check that it may: the piece
    /// moves (or promotes), whatever stood on the target is gone, and a king going two files
    /// takes its rook along.
    pub fn after_premove(&self, mv: Move) -> Self {
        let mut after = *self;
        let Some(piece) = self.piece_at(mv.from) else {
            return after;
        };
        after.set(mv.from, None);
        after.set(mv.to, Some(placed(piece, mv)));
        if piece.kind == PieceKind::King && mv.from.file().abs_diff(mv.to.file()) == 2 {
            move_castling_rook(&mut after, piece.side, mv);
        }
        after
    }
}

fn king_targets(from: Square, side: Side) -> Vec<Square> {
    let mut targets: Vec<Square> = steps(from, &KING_STEPS).collect();
    if from == home(4, side) {
        targets.extend(CASTLE_FILES.map(|file| home(file, side)));
    }
    targets
}

/// One or two squares forward (two only from the start rank), and the two capture squares.
fn pawn_targets(from: Square, side: Side) -> Vec<Square> {
    let forward = side.forward();
    let mut targets: Vec<Square> = [(0, forward), (-1, forward), (1, forward)]
        .into_iter()
        .filter_map(|(files, ranks)| from.offset(files, ranks))
        .collect();
    if from.rank() == side.back_rank().abs_diff(1) {
        targets.extend(from.offset(0, 2 * forward));
    }
    targets
}

fn steps(from: Square, offsets: &'static [(i8, i8)]) -> impl Iterator<Item = Square> {
    offsets
        .iter()
        .filter_map(move |&(files, ranks)| from.offset(files, ranks))
}

/// Every square along each direction, to the edge of the board.
fn rays(from: Square, directions: &'static [(i8, i8)]) -> impl Iterator<Item = Square> {
    directions
        .iter()
        .flat_map(move |&(files, ranks)| ray(from, files, ranks))
}

fn ray(from: Square, files: i8, ranks: i8) -> impl Iterator<Item = Square> {
    std::iter::successors(from.offset(files, ranks), move |square| {
        square.offset(files, ranks)
    })
}

/// The piece that lands: the mover itself, or what a pawn promotes to.
fn placed(piece: Piece, mv: Move) -> Piece {
    mv.promotion
        .map_or(piece, |kind| Piece::new(piece.side, kind))
}

/// The rook jumps to the square the king passed: h→f on the king side, a→d on the queen side.
fn move_castling_rook(board: &mut Board, side: Side, mv: Move) {
    let (from_file, to_file) = if mv.to.file() == 6 { (7, 5) } else { (0, 3) };
    if board.piece_at(home(from_file, side)) == Some(Piece::new(side, PieceKind::Rook)) {
        board.set(home(from_file, side), None);
        board.set(home(to_file, side), Some(Piece::new(side, PieceKind::Rook)));
    }
}

fn home(file: u8, side: Side) -> Square {
    Square::new(file, side.back_rank()).expect("a back-rank square")
}

#[cfg(test)]
mod tests {
    use crate::{Board, Move, Piece, PieceKind, Position, Side, Square};

    fn sq(name: &str) -> Square {
        name.parse().expect("a square")
    }

    fn names(squares: &[Square]) -> Vec<String> {
        squares.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_bishops_ray_runs_through_the_pieces_in_its_way() {
        let targets = Board::starting().premove_targets(sq("c1"));
        assert_eq!(names(&targets), ["b2", "d2", "a3", "e3", "f4", "g5", "h6"]);
    }

    #[test]
    fn a_pawn_on_its_start_rank_has_four_targets() {
        let targets = Board::starting().premove_targets(sq("e2"));
        assert_eq!(names(&targets), ["d3", "e3", "f3", "e4"]);
        let black = Board::starting().premove_targets(sq("e7"));
        assert_eq!(names(&black), ["e5", "d6", "e6", "f6"]);
    }

    #[test]
    fn the_king_at_home_lists_both_castling_squares() {
        let targets = Board::starting().premove_targets(sq("e1"));
        assert!(targets.contains(&sq("c1")));
        assert!(targets.contains(&sq("g1")));
        assert!(targets.contains(&sq("d2")));
        assert!(Board::starting().premove_targets(sq("e4")).is_empty());
    }

    #[test]
    fn a_premoved_castling_moves_the_rook_too() {
        let board = *"r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"
            .parse::<Position>()
            .expect("valid")
            .board();
        let after = board.after_premove(Move::new(sq("e1"), sq("g1")));
        assert_eq!(
            after.piece_at(sq("g1")),
            Some(Piece::new(Side::White, PieceKind::King))
        );
        assert_eq!(
            after.piece_at(sq("f1")),
            Some(Piece::new(Side::White, PieceKind::Rook))
        );
        assert_eq!(after.piece_at(sq("h1")), None);
    }

    #[test]
    fn a_premoved_promotion_lands_as_the_chosen_piece() {
        let board = *"8/4P3/8/8/8/8/8/k6K w - - 0 1"
            .parse::<Position>()
            .expect("valid")
            .board();
        let after = board.after_premove(Move::promoting(sq("e7"), sq("e8"), PieceKind::Knight));
        assert_eq!(
            after.piece_at(sq("e8")),
            Some(Piece::new(Side::White, PieceKind::Knight))
        );
        assert_eq!(after.piece_at(sq("e7")), None);
    }
}
