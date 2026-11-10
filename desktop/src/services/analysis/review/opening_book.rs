//! The opening table: positions a game reaches while it is still following known theory.
use std::{collections::HashSet, sync::LazyLock};

use chess_domain::{Move, PieceKind, Position};

/// One line per opening, in SAN from the start; lines starting with `#` are notes.
const OPENINGS: &str = include_str!("../../../../assets/review/openings.txt");

/// Every position along every line, by its key.
static BOOK: LazyLock<HashSet<String>> = LazyLock::new(|| {
    OPENINGS
        .lines()
        .filter_map(line_positions)
        .flatten()
        .map(|position| position_key(&position))
        .collect()
});

/// Builds the opening table now, off the UI thread, so the first `is_book` later costs nothing.
pub fn warm_up() {
    LazyLock::force(&BOOK);
}

/// Whether the position is on one of the opening lines.
pub fn is_book(position: &Position) -> bool {
    BOOK.contains(&position_key(position))
}

/// The positions after each move of a line; `None` for a note, a blank line or an illegal move.
fn line_positions(line: &str) -> Option<Vec<Position>> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let mut position = Position::starting();
    let mut reached = Vec::new();
    for san in line.split_whitespace() {
        let mv = move_named(&position, san)?;
        position = position.play(mv).ok()?;
        reached.push(position);
    }
    Some(reached)
}

/// The legal move written `san`, check marks aside.
pub(super) fn move_named(position: &Position, san: &str) -> Option<Move> {
    let wanted = san.trim_end_matches(['+', '#']);
    position.legal_moves().into_iter().find(|&mv| {
        position
            .san(mv)
            .is_ok_and(|written| written.trim_end_matches(['+', '#']) == wanted)
    })
}

/// The FEN without its move counters: the same position reached by another order is the same.
/// The en passant square only counts when a pawn can really take there, as in an EPD, so 1. d4
/// d5 2. Nf3 and 1. Nf3 d5 2. d4 are one position.
pub(super) fn position_key(position: &Position) -> String {
    let fen = position.to_string();
    let mut fields: Vec<&str> = fen.split_whitespace().take(4).collect();
    if !can_take_en_passant(position)
        && let Some(en_passant) = fields.get_mut(3)
    {
        *en_passant = "-";
    }
    fields.join(" ")
}

/// Whether a pawn of the side to move can take en passant right now.
fn can_take_en_passant(position: &Position) -> bool {
    position.en_passant().is_some_and(|square| {
        position.legal_moves().into_iter().any(|mv| {
            mv.to == square
                && position
                    .board()
                    .piece_at(mv.from)
                    .is_some_and(|piece| piece.kind == PieceKind::Pawn)
        })
    })
}

#[cfg(test)]
mod tests {
    use chess_domain::Position;

    use super::{OPENINGS, is_book, line_positions, move_named, position_key};

    fn after(sans: &[&str]) -> Position {
        sans.iter().fold(Position::starting(), |position, san| {
            let mv = move_named(&position, san).expect("a legal move");
            position.play(mv).expect("played")
        })
    }

    #[test]
    fn every_line_of_the_table_is_legal() {
        for line in OPENINGS.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            assert!(line_positions(line).is_some(), "not legal: {line}");
        }
    }

    #[test]
    fn known_openings_are_book_and_others_are_not() {
        assert!(is_book(&after(&["e4", "e5", "Nf3"])));
        assert!(is_book(&after(&["d4", "Nf6", "c4", "e6", "Nc3", "Bb4"])));
        assert!(!is_book(&after(&["e4", "e5", "Qh5"])));
        assert!(!is_book(&Position::starting()), "the start is not a move");
    }

    #[test]
    fn the_en_passant_square_counts_only_when_a_pawn_can_take_there() {
        let key = |sans: &[&str]| position_key(&after(sans));
        assert!(key(&["e4"]).ends_with(" -"), "no black pawn can take on e3");
        assert_eq!(key(&["Nf3", "d5", "d4"]), key(&["d4", "d5", "Nf3"]));
        assert!(
            key(&["e4", "d5", "e5", "f5"]).ends_with(" f6"),
            "the e5 pawn can take on f6"
        );
    }
}
