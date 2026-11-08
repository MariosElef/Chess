use chess_domain::{GameStatus, Position, Side};

/// The line under the title: whose turn it is, a check warning, or how the game ended.
pub fn status_text(position: &Position) -> String {
    match position.status() {
        GameStatus::Ongoing => turn_text(position),
        GameStatus::Checkmate { winner } => format!("Checkmate — {} wins", side_name(winner)),
        GameStatus::Stalemate => "Stalemate — it's a draw".to_owned(),
    }
}

pub fn is_over(position: &Position) -> bool {
    position.status() != GameStatus::Ongoing
}

fn turn_text(position: &Position) -> String {
    let side = side_name(position.side_to_move());
    if position.is_check() {
        format!("{side} to move — check!")
    } else {
        format!("{side} to move")
    }
}

pub(super) fn side_name(side: Side) -> &'static str {
    match side {
        Side::White => "White",
        Side::Black => "Black",
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::{Move, Position};

    use super::{is_over, status_text};

    fn after(moves: &[(&str, &str)]) -> Position {
        moves
            .iter()
            .fold(Position::starting(), |position, (from, to)| {
                let mv = Move::new(
                    from.parse().expect("a square"),
                    to.parse().expect("a square"),
                );
                position.play(mv).expect("legal")
            })
    }

    #[test]
    fn the_game_starts_with_white_to_move() {
        let start = Position::starting();
        assert_eq!(status_text(&start), "White to move");
        assert!(!is_over(&start));
    }

    #[test]
    fn a_check_is_announced() {
        let position = after(&[("e2", "e4"), ("f7", "f6"), ("d1", "h5")]);
        assert_eq!(status_text(&position), "Black to move — check!");
    }

    #[test]
    fn checkmate_names_the_winner() {
        let position = after(&[("f2", "f3"), ("e7", "e5"), ("g2", "g4"), ("d8", "h4")]);
        assert_eq!(status_text(&position), "Checkmate — Black wins");
        assert!(is_over(&position));
    }

    #[test]
    fn stalemate_is_a_draw() {
        let position: Position = "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1".parse().expect("valid");
        assert_eq!(status_text(&position), "Stalemate — it's a draw");
        assert!(is_over(&position));
    }
}
