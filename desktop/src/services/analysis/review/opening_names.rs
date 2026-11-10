//! The openings' names: Lichess's chess-openings list, every named line by the position it
//! reaches, so a game that gets there by another order is named the same.
use std::{collections::HashMap, sync::LazyLock};

use chess_domain::{Game, Position};

use super::opening_book::{move_named, position_key};
use crate::models::analysis::Opening;

/// One opening per line after the header: its ECO code, its name and its moves, by tabs.
const NAMES: &str = include_str!("../../../../assets/review/opening_names.tsv");

/// Every named position, by its key; where two lines reach one position, the first names it.
static NAMED: LazyLock<HashMap<String, Opening>> = LazyLock::new(|| {
    let mut named = HashMap::new();
    for (opening, position) in NAMES.lines().skip(1).filter_map(named_line) {
        named.entry(position_key(&position)).or_insert(opening);
    }
    named
});

/// Builds the names now, off the UI thread, so naming a game later costs nothing.
pub fn warm_up_names() {
    LazyLock::force(&NAMED);
}

/// At every position of the game, its opening so far: the name of the last named position it
/// had reached, as Lichess names a game; none at the start, before a move.
pub fn openings_by_position(game: &Game) -> Vec<Option<Opening>> {
    let mut so_far: Option<Opening> = None;
    game.positions()
        .iter()
        .enumerate()
        .map(|(index, position)| {
            if index > 0
                && let Some(named) = NAMED.get(&position_key(position))
            {
                so_far = Some(named.clone());
            }
            so_far.clone()
        })
        .collect()
}

/// The game's opening: the name of the last named position it reached.
#[cfg(test)]
fn opening_of(game: &Game) -> Option<Opening> {
    openings_by_position(game).pop().flatten()
}

/// A line's opening and the position its moves reach; `None` for a line that does not read.
fn named_line(line: &str) -> Option<(Opening, Position)> {
    let mut fields = line.split('\t');
    let (eco, name, moves) = (fields.next()?, fields.next()?, fields.next()?);
    let position = moves
        .split_whitespace()
        .filter(|token| !token.ends_with('.'))
        .try_fold(Position::starting(), |position, san| {
            position.play(move_named(&position, san)?).ok()
        })?;
    let opening = Opening {
        eco: eco.to_owned(),
        name: name.to_owned(),
    };
    Some((opening, position))
}

#[cfg(test)]
mod tests {
    use chess_domain::Game;

    use super::{NAMES, named_line, opening_of, openings_by_position};

    fn game(ucis: &[&str]) -> Game {
        let mut game = Game::new();
        for uci in ucis {
            game.play(uci.parse().expect("a move")).expect("legal");
        }
        game
    }

    #[test]
    fn every_line_of_the_list_reads() {
        for line in NAMES.lines().skip(1) {
            assert!(named_line(line).is_some(), "{line}");
        }
    }

    #[test]
    fn a_game_is_named_by_the_deepest_named_position_it_reached() {
        // 1. e4 c5 2. Nf3 d6 3. d4 cxd4 4. Nxd4 Nf6 5. Nc3 a6, then a move of its own.
        let najdorf = game(&[
            "e2e4", "c7c5", "g1f3", "d7d6", "d2d4", "c5d4", "f3d4", "g8f6", "b1c3", "a7a6", "h2h3",
        ]);
        let opening = opening_of(&najdorf).expect("named");
        assert_eq!(
            opening.name,
            "Sicilian Defense: Najdorf Variation, Adams Attack"
        );
        let left_early = game(&["e2e4", "c7c5", "a2a3", "h7h6"]);
        assert_eq!(
            opening_of(&left_early).expect("named").name,
            "Sicilian Defense: Mengarini Variation"
        );
    }

    #[test]
    fn another_move_order_reaching_a_named_position_has_its_name() {
        // 1. Nf3 d5 2. d4 reaches 1. d4 d5 2. Nf3's position.
        let by_knight = game(&["g1f3", "d7d5", "d2d4"]);
        let by_pawn = game(&["d2d4", "d7d5", "g1f3"]);
        assert_eq!(opening_of(&by_knight), opening_of(&by_pawn));
    }

    #[test]
    fn a_game_without_moves_has_no_opening() {
        assert_eq!(opening_of(&Game::new()), None);
    }

    #[test]
    fn the_opening_grows_move_by_move_and_stays_once_the_game_leaves_it() {
        let game = game(&["e2e4", "c7c5", "a2a3", "h7h6"]);
        let names: Vec<Option<String>> = openings_by_position(&game)
            .into_iter()
            .map(|opening| opening.map(|opening| opening.name))
            .collect();
        assert_eq!(names[0], None, "nothing before the first move");
        assert_eq!(names[1].as_deref(), Some("King's Pawn Game"));
        assert_eq!(names[2].as_deref(), Some("Sicilian Defense"));
        assert_eq!(
            names[3].as_deref(),
            Some("Sicilian Defense: Mengarini Variation")
        );
        assert_eq!(names[4], names[3], "h6 is no opening: the name stays");
    }
}
