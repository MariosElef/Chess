//! Standard Algebraic Notation, the way people write moves: `e4`, `Nf3`, `exd5`, `O-O`, `e8=Q+`.
use crate::{GameStatus, IllegalMove, Move, Piece, PieceKind, Position, Square};

impl Position {
    /// How `mv` is written in this position, check (`+`) and mate (`#`) included.
    pub fn san(&self, mv: Move) -> Result<String, IllegalMove> {
        let after = self.play(mv)?;
        let piece = self
            .board()
            .piece_at(mv.from)
            .expect("a legal move starts on a piece");
        Ok(format!(
            "{}{}",
            self.move_text(piece, mv),
            check_suffix(&after)
        ))
    }

    /// A line of moves in notation, as an engine's suggestion is shown; it stops at the first
    /// move that is not legal where it would be played.
    pub fn san_line(&self, moves: &[Move]) -> Vec<String> {
        let mut position = *self;
        let mut sans = Vec::new();
        for &mv in moves {
            let (Ok(san), Ok(next)) = (position.san(mv), position.play(mv)) else {
                break;
            };
            sans.push(san);
            position = next;
        }
        sans
    }

    fn move_text(&self, piece: Piece, mv: Move) -> String {
        if is_castle(piece, mv) {
            return castle_text(mv).to_owned();
        }
        match piece.kind {
            PieceKind::Pawn => self.pawn_text(piece, mv),
            PieceKind::King
            | PieceKind::Queen
            | PieceKind::Rook
            | PieceKind::Bishop
            | PieceKind::Knight => self.piece_text(piece, mv),
        }
    }

    /// A pawn names its file only when it captures: `e4`, `exd5`, `e8=Q`.
    fn pawn_text(&self, piece: Piece, mv: Move) -> String {
        let capture = if self.is_capture(piece, mv) {
            format!("{}x", file_letter(mv.from))
        } else {
            String::new()
        };
        format!("{capture}{}{}", mv.to, promotion_text(mv))
    }

    /// Letter, disambiguation, capture and target: `Nf3`, `Nbd2`, `R1a3`, `Qxe7`.
    fn piece_text(&self, piece: Piece, mv: Move) -> String {
        let capture = if self.is_capture(piece, mv) { "x" } else { "" };
        format!(
            "{}{}{capture}{}",
            piece_letter(piece.kind),
            self.disambiguation(piece, mv),
            mv.to
        )
    }

    /// The other squares from which the same kind of piece could also reach the target.
    fn rivals(&self, piece: Piece, mv: Move) -> Vec<Square> {
        self.legal_moves()
            .into_iter()
            .filter(|other| other.to == mv.to && other.from != mv.from)
            .filter(|other| self.board().piece_at(other.from) == Some(piece))
            .map(|other| other.from)
            .collect()
    }

    /// The file if it tells the pieces apart, else the rank, else both.
    fn disambiguation(&self, piece: Piece, mv: Move) -> String {
        let rivals = self.rivals(piece, mv);
        if rivals.is_empty() {
            return String::new();
        }
        let shares_file = rivals.iter().any(|square| square.file() == mv.from.file());
        let shares_rank = rivals.iter().any(|square| square.rank() == mv.from.rank());
        match (shares_file, shares_rank) {
            (false, _) => file_letter(mv.from).to_string(),
            (true, false) => rank_digit(mv.from).to_string(),
            (true, true) => mv.from.to_string(),
        }
    }
}

fn is_castle(piece: Piece, mv: Move) -> bool {
    piece.kind == PieceKind::King && mv.from.file().abs_diff(mv.to.file()) == 2
}

fn castle_text(mv: Move) -> &'static str {
    if mv.to.file() == 6 { "O-O" } else { "O-O-O" }
}

fn check_suffix(after: &Position) -> &'static str {
    match after.status() {
        GameStatus::Checkmate { .. } => "#",
        GameStatus::Ongoing | GameStatus::Stalemate if after.is_check() => "+",
        GameStatus::Ongoing | GameStatus::Stalemate => "",
    }
}

fn promotion_text(mv: Move) -> String {
    mv.promotion
        .map_or_else(String::new, |kind| format!("={}", piece_letter(kind)))
}

fn piece_letter(kind: PieceKind) -> &'static str {
    match kind {
        PieceKind::King => "K",
        PieceKind::Queen => "Q",
        PieceKind::Rook => "R",
        PieceKind::Bishop => "B",
        PieceKind::Knight => "N",
        PieceKind::Pawn => "",
    }
}

fn file_letter(square: Square) -> char {
    char::from(b'a' + square.file())
}

fn rank_digit(square: Square) -> char {
    char::from(b'1' + square.rank())
}

#[cfg(test)]
mod tests {
    use crate::{Move, PieceKind, Position, Square};

    fn sq(name: &str) -> Square {
        name.parse().expect("a square")
    }

    fn fen(text: &str) -> Position {
        text.parse().expect("a valid FEN")
    }

    /// Plays the moves from the start and returns how each was written.
    fn written(moves: &[(&str, &str)]) -> Vec<String> {
        let mut position = Position::starting();
        let mut sans = Vec::new();
        for (from, to) in moves {
            let mv = Move::new(sq(from), sq(to));
            sans.push(position.san(mv).expect("legal"));
            position = position.play(mv).expect("legal");
        }
        sans
    }

    #[test]
    fn pawns_and_pieces_in_an_opening() {
        let sans = written(&[
            ("e2", "e4"),
            ("e7", "e5"),
            ("g1", "f3"),
            ("b8", "c6"),
            ("f1", "b5"),
        ]);
        assert_eq!(sans, ["e4", "e5", "Nf3", "Nc6", "Bb5"]);
    }

    #[test]
    fn captures_and_check() {
        let sans = written(&[("e2", "e4"), ("d7", "d5"), ("e4", "d5"), ("d8", "d5")]);
        assert_eq!(sans, ["e4", "d5", "exd5", "Qxd5"]);
        let check = written(&[("e2", "e4"), ("f7", "f6"), ("d1", "h5")]);
        assert_eq!(check[2], "Qh5+");
    }

    #[test]
    fn checkmate_gets_a_hash() {
        let sans = written(&[("f2", "f3"), ("e7", "e5"), ("g2", "g4"), ("d8", "h4")]);
        assert_eq!(sans[3], "Qh4#");
    }

    #[test]
    fn castling_both_ways() {
        let position = fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
        assert_eq!(
            position.san(Move::new(sq("e1"), sq("g1"))),
            Ok("O-O".to_owned())
        );
        assert_eq!(
            position.san(Move::new(sq("e1"), sq("c1"))),
            Ok("O-O-O".to_owned())
        );
    }

    #[test]
    fn two_knights_are_told_apart_by_file() {
        let position = fen("4k3/8/8/8/8/8/8/N1N1K3 w - - 0 1");
        assert_eq!(
            position.san(Move::new(sq("a1"), sq("b3"))),
            Ok("Nab3".to_owned())
        );
    }

    #[test]
    fn two_rooks_on_a_file_are_told_apart_by_rank() {
        let position = fen("4k3/8/8/R7/8/8/8/R3K3 w - - 0 1");
        assert_eq!(
            position.san(Move::new(sq("a1"), sq("a3"))),
            Ok("R1a3".to_owned())
        );
    }

    #[test]
    fn promotion_names_the_new_piece() {
        let position = fen("8/4P3/8/8/8/8/8/k6K w - - 0 1");
        let mv = Move::promoting(sq("e7"), sq("e8"), PieceKind::Queen);
        assert_eq!(position.san(mv), Ok("e8=Q".to_owned()));
    }

    #[test]
    fn en_passant_is_written_as_a_capture() {
        let position = fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1");
        assert_eq!(
            position.san(Move::new(sq("e5"), sq("d6"))),
            Ok("exd6".to_owned())
        );
    }

    #[test]
    fn a_line_is_written_until_its_first_illegal_move() {
        let moves = [("e2", "e4"), ("e7", "e5"), ("e4", "e5"), ("g1", "f3")]
            .map(|(from, to)| Move::new(sq(from), sq(to)));
        assert_eq!(Position::starting().san_line(&moves), ["e4", "e5"]);
    }

    #[test]
    fn an_illegal_move_has_no_notation() {
        assert!(
            Position::starting()
                .san(Move::new(sq("e2"), sq("e5")))
                .is_err()
        );
    }
}
