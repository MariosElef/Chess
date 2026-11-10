use chess_domain::{Game, Outcome};

use super::side_label;
use crate::i18n::tr;

/// The line under the title: whose turn it is, a check or draw offer, or how the game ended.
pub fn status_text(game: &Game) -> String {
    match game.outcome() {
        Some(outcome) => outcome_text(outcome),
        None => turn_text(game),
    }
}

fn outcome_text(outcome: Outcome) -> String {
    match outcome {
        Outcome::Checkmate { winner } => {
            tr!("play.status.checkmate", winner = side_label(winner))
        }
        Outcome::Resignation { winner } => tr!(
            "play.status.resigned",
            loser = side_label(winner.opponent()),
            winner = side_label(winner)
        ),
        Outcome::Timeout { winner } => tr!(
            "play.status.out_of_time",
            loser = side_label(winner.opponent()),
            winner = side_label(winner)
        ),
        Outcome::Stalemate => tr!("play.status.stalemate"),
        Outcome::DrawAgreement => tr!("play.status.draw_agreed"),
        Outcome::InsufficientMaterial => tr!("play.status.draw_insufficient_material"),
        Outcome::Repetition => tr!("play.status.draw_repetition"),
        Outcome::FiftyMoves => tr!("play.status.draw_fifty_moves"),
    }
}

fn turn_text(game: &Game) -> String {
    let live = game.live();
    let side = side_label(live.side_to_move());
    let status = if live.is_check() {
        tr!("play.status.to_move_check", side = side)
    } else {
        tr!("play.status.to_move", side = side)
    };
    match game.draw_offer() {
        Some(by) => tr!(
            "play.status.with_offer",
            status = status,
            offer = tr!("play.status.offers_draw", side = side_label(by))
        ),
        None => status,
    }
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
        assert_eq!(status_text(&Game::from_position(position)), "Stalemate");
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
