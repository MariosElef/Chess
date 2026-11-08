use chess_domain::{Game, Position};

use crate::models::analysis::Pasted;

/// A FEN or a PGN, whichever the text is; else why it is neither.
pub fn read_pasted(text: &str) -> Result<Pasted, String> {
    let text = text.trim();
    if let Ok(position) = text.parse::<Position>() {
        return Ok(Pasted::Position(position));
    }
    match Game::from_pgn(text) {
        Ok(game) => Ok(Pasted::Game(game)),
        Err(error) if looks_like_pgn(text) => Err(format!("Not a valid PGN: {error}")),
        Err(_) => {
            let error = text
                .parse::<Position>()
                .err()
                .map(|error| error.to_string());
            Err(format!("Not a valid FEN: {}", error.unwrap_or_default()))
        }
    }
}

fn looks_like_pgn(text: &str) -> bool {
    text.contains('[') || text.contains("1.")
}

#[cfg(test)]
mod tests {
    use super::read_pasted;
    use crate::models::analysis::Pasted;

    #[test]
    fn a_fen_is_a_position_and_a_pgn_a_game() {
        let fen = read_pasted("8/8/8/8/8/8/8/k6K w - - 0 1").expect("a FEN");
        assert!(matches!(fen, Pasted::Position(_)));
        let pgn = read_pasted("[Event \"x\"]\n1. e4 e5 2. Nf3 *").expect("a PGN");
        let Pasted::Game(game) = pgn else {
            panic!("a game");
        };
        assert_eq!(game.sans(), ["e4", "e5", "Nf3"]);
    }

    #[test]
    fn neither_says_why_for_what_it_looks_like() {
        let pgn = read_pasted("1. e4 e5 2. Ke3").expect_err("not legal");
        assert_eq!(pgn, "Not a valid PGN: 2. Ke3 is not a legal move there");
        let fen = read_pasted("nonsense").expect_err("not a FEN");
        assert!(fen.starts_with("Not a valid FEN: "), "{fen}");
    }
}
