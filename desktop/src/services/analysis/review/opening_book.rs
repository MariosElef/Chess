//! The opening table: positions a game reaches while it is still following known theory.
use std::{collections::HashSet, sync::LazyLock};

use chess_domain::{Move, Position};

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

/// Builds the table now, so the first review does not wait for it when it finishes.
pub fn warm_opening_book() {
    LazyLock::force(&BOOK);
}

/// Whether the position is on one of the opening lines.
pub fn is_book(position: &Position) -> bool {
    BOOK.contains(&position_key(position))
}

/// The positions after each move of a line; `None` for a note, a blank line, or a move that is
/// not legal where it stands.
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
fn move_named(position: &Position, san: &str) -> Option<Move> {
    let wanted = san.trim_end_matches(['+', '#']);
    position.legal_moves().into_iter().find(|&mv| {
        position
            .san(mv)
            .is_ok_and(|written| written.trim_end_matches(['+', '#']) == wanted)
    })
}

/// The FEN without its move counters: the same position reached by another order is the same.
fn position_key(position: &Position) -> String {
    let fen = position.to_string();
    fen.split_whitespace().take(4).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use chess_domain::Position;

    use super::{OPENINGS, is_book, line_positions, move_named};

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
}
