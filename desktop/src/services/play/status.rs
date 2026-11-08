use chess_domain::{Game, Outcome};

use crate::services::board::side_name;

/// The line under the title: whose turn it is, a check or draw offer, or how the game ended.
pub fn status_text(game: &Game) -> String {
    match game.outcome() {
        Some(outcome) => outcome_text(outcome),
        None => turn_text(game),
    }
}

fn outcome_text(outcome: Outcome) -> String {
    match outcome {
        Outcome::Checkmate { winner } => format!("Checkmate — {} wins", side_name(winner)),
        Outcome::Resignation { winner } => format!(
            "{} resigned — {} wins",
            side_name(winner.opponent()),
            side_name(winner)
        ),
        Outcome::Timeout { winner } => format!(
            "{} ran out of time — {} wins",
            side_name(winner.opponent()),
            side_name(winner)
        ),
        Outcome::Stalemate => "Stalemate — it's a draw".to_owned(),
        Outcome::DrawAgreement => "Draw agreed".to_owned(),
        Outcome::InsufficientMaterial => "Draw — neither side can checkmate".to_owned(),
        Outcome::Repetition => "Draw by threefold repetition".to_owned(),
        Outcome::FiftyMoves => "Draw by the fifty-move rule".to_owned(),
    }
}

fn turn_text(game: &Game) -> String {
    let live = game.live();
    let side = side_name(live.side_to_move());
    let check = if live.is_check() { " — check!" } else { "" };
    let offer = game.draw_offer().map_or_else(String::new, |by| {
        format!(" · {} offers a draw", side_name(by))
    });
    format!("{side} to move{check}{offer}")
}

#[cfg(test)]
mod tests {
    use chess_domain::{Game, Move, Position, Side};

    use super::status_text;

    fn after(moves: &[(&str, &str)]) -> Game {
        let mut game = Game::new();
        for (from, to) in moves {
            let mv = Move::new(
                from.parse().expect("a square"),
                to.parse().expect("a square"),
            );
            game.play(mv).expect("legal");
        }
        game
    }

    #[test]
    fn the_game_starts_with_white_to_move() {
        assert_eq!(status_text(&Game::new()), "White to move");
    }

    #[test]
    fn a_check_is_announced() {
        let game = after(&[("e2", "e4"), ("f7", "f6"), ("d1", "h5")]);
        assert_eq!(status_text(&game), "Black to move — check!");
    }

    #[test]
    fn checkmate_names_the_winner() {
        let game = after(&[("f2", "f3"), ("e7", "e5"), ("g2", "g4"), ("d8", "h4")]);
        assert_eq!(status_text(&game), "Checkmate — Black wins");
    }

    #[test]
    fn stalemate_is_a_draw() {
        let position: Position = "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1".parse().expect("valid");
        assert_eq!(
            status_text(&Game::from_position(position)),
            "Stalemate — it's a draw"
        );
    }

    #[test]
    fn resignation_and_draw_offers_are_announced() {
        let mut game = Game::new();
        game.offer_draw(Side::White).expect("ongoing");
        assert_eq!(status_text(&game), "White to move · White offers a draw");
        game.resign(Side::White).expect("ongoing");
        assert_eq!(status_text(&game), "White resigned — Black wins");
    }
}
