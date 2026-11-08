use crate::{
    Board, CastlingRights, GameStatus, IllegalMove, Move, Piece, PieceKind, Side, Square,
    models::moves::rules,
};

/// Everything the rules need to know about a moment in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    board: Board,
    side_to_move: Side,
    castling: CastlingRights,
    en_passant: Option<Square>,
    counters: MoveCounters,
}

/// The two numbers at the end of a FEN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveCounters {
    /// Half-moves since the last capture or pawn move; the fifty-move rule is 100 of them.
    pub halfmove_clock: u32,
    /// Starts at 1 and goes up after each of Black's moves.
    pub fullmove_number: u32,
}

impl MoveCounters {
    pub const START: Self = Self {
        halfmove_clock: 0,
        fullmove_number: 1,
    };
}

impl Position {
    pub fn starting() -> Self {
        Self::new(
            Board::starting(),
            Side::White,
            CastlingRights::ALL,
            None,
            MoveCounters::START,
        )
    }

    pub(crate) fn new(
        board: Board,
        side_to_move: Side,
        castling: CastlingRights,
        en_passant: Option<Square>,
        counters: MoveCounters,
    ) -> Self {
        Self {
            board,
            side_to_move,
            castling,
            en_passant,
            counters,
        }
    }

    pub fn counters(&self) -> MoveCounters {
        self.counters
    }

    /// The same position for repetition: pieces, side to move, castling, en passant; no counters.
    pub fn is_same_position(&self, other: &Self) -> bool {
        self.board == other.board
            && self.side_to_move == other.side_to_move
            && self.castling == other.castling
            && self.en_passant == other.en_passant
    }

    /// Fifty moves each without a capture or pawn move; either player may then claim a draw.
    pub fn is_fifty_move_draw(&self) -> bool {
        self.counters.halfmove_clock >= 100
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn side_to_move(&self) -> Side {
        self.side_to_move
    }

    pub fn castling(&self) -> CastlingRights {
        self.castling
    }

    /// The square a pawn may capture onto en passant, right after a double step.
    pub fn en_passant(&self) -> Option<Square> {
        self.en_passant
    }

    pub fn is_check(&self) -> bool {
        self.king_attacked(self.side_to_move)
    }

    pub fn legal_moves(&self) -> Vec<Move> {
        let side = self.side_to_move;
        rules::pseudo_legal_moves(self)
            .into_iter()
            .filter(|&mv| !self.apply(mv).king_attacked(side))
            .collect()
    }

    /// The legal moves of the piece on `from`; none for an empty square or the side not to move.
    pub fn legal_moves_from(&self, from: Square) -> Vec<Move> {
        self.legal_moves()
            .into_iter()
            .filter(|mv| mv.from == from)
            .collect()
    }

    pub fn play(&self, mv: Move) -> Result<Self, IllegalMove> {
        if self.legal_moves().contains(&mv) {
            Ok(self.apply(mv))
        } else {
            Err(IllegalMove(mv))
        }
    }

    pub fn status(&self) -> GameStatus {
        match (self.legal_moves().is_empty(), self.is_check()) {
            (false, _) => GameStatus::Ongoing,
            (true, true) => GameStatus::Checkmate {
                winner: self.side_to_move.opponent(),
            },
            (true, false) => GameStatus::Stalemate,
        }
    }

    fn king_attacked(&self, side: Side) -> bool {
        self.board
            .king_of(side)
            .is_some_and(|king| rules::is_attacked(&self.board, king, side.opponent()))
    }

    /// Plays `mv` without asking whether it is legal; `legal_moves` and `play` do the asking.
    fn apply(&self, mv: Move) -> Self {
        let piece = self
            .board
            .piece_at(mv.from)
            .expect("a generated move starts on a piece");
        let mut next = *self;
        next.board.set(mv.from, None);
        if self.is_en_passant_capture(piece, mv) {
            next.board.set(en_passant_victim(mv), None);
        }
        if is_castle(piece, mv) {
            move_castling_rook(&mut next.board, piece.side, mv);
        }
        next.board.set(mv.to, Some(placed_piece(piece, mv)));
        next.castling = castling_after(self.castling, piece, mv);
        next.en_passant = en_passant_after(piece, mv);
        next.counters = counters_after(self.counters, piece, self.is_capture(piece, mv));
        next.side_to_move = piece.side.opponent();
        next
    }

    pub(crate) fn is_capture(&self, piece: Piece, mv: Move) -> bool {
        self.board.piece_at(mv.to).is_some() || self.is_en_passant_capture(piece, mv)
    }

    fn is_en_passant_capture(&self, piece: Piece, mv: Move) -> bool {
        piece.kind == PieceKind::Pawn
            && Some(mv.to) == self.en_passant
            && mv.from.file() != mv.to.file()
    }
}

/// The pawn taken en passant stands beside the capturing pawn, not on the square it lands on.
fn en_passant_victim(mv: Move) -> Square {
    Square::new(mv.to.file(), mv.from.rank()).expect("beside the capturing pawn")
}

/// Castling is the only king move that crosses two files.
fn is_castle(piece: Piece, mv: Move) -> bool {
    piece.kind == PieceKind::King && mv.from.file().abs_diff(mv.to.file()) == 2
}

/// The rook jumps to the square the king passed: h→f on the king side, a→d on the queen side.
fn move_castling_rook(board: &mut Board, side: Side, mv: Move) {
    let (from_file, to_file) = if mv.to.file() == 6 { (7, 5) } else { (0, 3) };
    let rank = side.back_rank();
    let from = Square::new(from_file, rank).expect("a corner");
    let to = Square::new(to_file, rank).expect("beside the king");
    board.set(from, None);
    board.set(to, Some(Piece::new(side, PieceKind::Rook)));
}

/// The piece that lands: the mover itself, or what a pawn promotes to.
fn placed_piece(piece: Piece, mv: Move) -> Piece {
    mv.promotion
        .map_or(piece, |kind| Piece::new(piece.side, kind))
}

fn castling_after(rights: CastlingRights, piece: Piece, mv: Move) -> CastlingRights {
    let mut rights = rights;
    if piece.kind == PieceKind::King {
        rights.revoke_side(piece.side);
    }
    rights.revoke_corner(mv.from);
    rights.revoke_corner(mv.to);
    rights
}

/// A capture or pawn move resets the half-move clock; Black's move ends a full move.
fn counters_after(counters: MoveCounters, piece: Piece, is_capture: bool) -> MoveCounters {
    MoveCounters {
        halfmove_clock: halfmove_clock_after(counters.halfmove_clock, piece, is_capture),
        fullmove_number: fullmove_number_after(counters.fullmove_number, piece.side),
    }
}

fn halfmove_clock_after(clock: u32, piece: Piece, is_capture: bool) -> u32 {
    if is_capture || piece.kind == PieceKind::Pawn {
        0
    } else {
        clock + 1
    }
}

fn fullmove_number_after(number: u32, mover: Side) -> u32 {
    match mover {
        Side::White => number,
        Side::Black => number + 1,
    }
}

/// After a pawn's double step, the square it skipped can be captured onto next move.
fn en_passant_after(piece: Piece, mv: Move) -> Option<Square> {
    let double_step = mv.from.rank().abs_diff(mv.to.rank()) == 2;
    if piece.kind == PieceKind::Pawn && double_step {
        mv.from.offset(0, piece.side.forward())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::Position;
    use crate::{GameStatus, IllegalMove, Move, Side, Square};

    fn mv(from: &str, to: &str) -> Move {
        Move::new(
            from.parse::<Square>().expect("a square"),
            to.parse::<Square>().expect("a square"),
        )
    }

    fn perft(position: &Position, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }
        position
            .legal_moves()
            .into_iter()
            .map(|mv| perft(&position.apply(mv), depth - 1))
            .sum()
    }

    fn fen(text: &str) -> Position {
        text.parse().expect("a valid FEN")
    }

    // Published move counts (chessprogramming.org "Perft Results"): any rule bug changes them.
    #[test]
    fn perft_from_the_start() {
        let start = Position::starting();
        assert_eq!([1, 2, 3].map(|depth| perft(&start, depth)), [20, 400, 8902]);
    }

    #[test]
    fn perft_kiwipete_castling_and_en_passant() {
        let position = fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        assert_eq!([1, 2].map(|depth| perft(&position, depth)), [48, 2039]);
    }

    #[test]
    fn perft_pins_and_en_passant_discovered_check() {
        let position = fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1");
        assert_eq!(
            [1, 2, 3].map(|depth| perft(&position, depth)),
            [14, 191, 2812]
        );
    }

    #[test]
    fn perft_promotions() {
        let position = fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1");
        assert_eq!(
            [1, 2, 3].map(|depth| perft(&position, depth)),
            [6, 264, 9467]
        );
    }

    #[test]
    fn fools_mate_is_checkmate_for_black() {
        let mut position = Position::starting();
        for (from, to) in [("f2", "f3"), ("e7", "e5"), ("g2", "g4"), ("d8", "h4")] {
            position = position.play(mv(from, to)).expect("legal");
        }
        assert!(position.is_check());
        assert_eq!(
            position.status(),
            GameStatus::Checkmate {
                winner: Side::Black
            }
        );
    }

    #[test]
    fn a_king_with_no_moves_and_no_check_is_stalemate() {
        let position = fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1");
        assert!(!position.is_check());
        assert_eq!(position.status(), GameStatus::Stalemate);
    }

    #[test]
    fn legal_moves_from_lists_one_piece() {
        let start = Position::starting();
        let knight: Vec<String> = start
            .legal_moves_from("g1".parse().expect("g1"))
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(knight, ["g1h3", "g1f3"]);
        assert!(start.legal_moves_from("e4".parse().expect("e4")).is_empty());
        assert!(start.legal_moves_from("e7".parse().expect("e7")).is_empty());
    }

    #[test]
    fn an_illegal_move_is_refused() {
        let start = Position::starting();
        assert_eq!(start.play(mv("e2", "e5")), Err(IllegalMove(mv("e2", "e5"))));
        assert_eq!(start.status(), GameStatus::Ongoing);
    }

    #[test]
    fn counters_follow_captures_pawn_moves_and_full_moves() {
        let mut position = Position::starting();
        for (from, to) in [("g1", "f3"), ("g8", "f6"), ("f3", "g1")] {
            position = position.play(mv(from, to)).expect("legal");
        }
        assert_eq!(position.counters().halfmove_clock, 3);
        assert_eq!(position.counters().fullmove_number, 2);
        let after_pawn = position.play(mv("e7", "e5")).expect("legal");
        assert_eq!(after_pawn.counters().halfmove_clock, 0);
        assert_eq!(after_pawn.counters().fullmove_number, 3);
    }

    #[test]
    fn a_double_step_writes_its_en_passant_square() {
        let position = Position::starting().play(mv("e2", "e4")).expect("legal");
        assert_eq!(
            position.to_string(),
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
        );
    }

    #[test]
    fn a_hundred_quiet_half_moves_is_the_fifty_move_rule() {
        assert!(!fen("8/8/8/8/8/8/8/k6K w - - 99 80").is_fifty_move_draw());
        assert!(fen("8/8/8/8/8/8/8/k6K w - - 100 80").is_fifty_move_draw());
    }

    #[test]
    fn castling_moves_the_rook_too() {
        let position = fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1")
            .play(mv("e1", "g1"))
            .expect("legal");
        let board = position.board();
        assert!(board.piece_at("h1".parse().expect("h1")).is_none());
        assert!(board.piece_at("f1".parse().expect("f1")).is_some());
        assert!(!position.castling().king_side(Side::White));
        assert!(!position.castling().queen_side(Side::White));
    }
}
