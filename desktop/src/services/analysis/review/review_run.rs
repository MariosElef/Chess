use chess_domain::{
    Game, GameStatus, Position, Side,
    engine::{Eval, SearchInfo},
};

use super::{
    accuracy::{move_accuracy, side_accuracy},
    classify::{MoveFacts, classify},
    game_rating::{FEWEST_MOVES, estimated_rating},
    opening_book::{is_book, warm_up},
    sacrifice::is_sacrifice,
};
use crate::{
    enums::analysis::MoveClass,
    models::analysis::{EngineLine, GameReview, MoveReview, PositionEval},
};

/// Moves after this many are never Book, whatever the table says.
const BOOK_PLIES: usize = 20;
/// A sacrifice is only looked for in moves this close to best: no other can be Brilliant.
const NEARLY_BEST: f64 = 0.02;

/// A review under way: the game's positions, the engine's word on each so far, and those being
/// searched, several at once by several engines. A finished position needs no search, its result
/// being its value. The opening table's moves are Book whatever the engine says, but their
/// positions are searched too: the review's graph shows the game from its first move.
#[derive(Debug, Clone)]
pub struct ReviewRun {
    game: Game,
    evals: Vec<Option<PositionEval>>,
    searching: Vec<usize>,
}

impl ReviewRun {
    /// Builds the opening table the first time, which takes a moment: call it off the UI thread.
    pub fn new(game: &Game) -> Self {
        warm_up();
        let evals = game.positions().iter().map(finished_eval).collect();
        Self {
            game: game.clone(),
            evals,
            searching: Vec::new(),
        }
    }

    /// The next position to search: its index in the game, 0 being the start, and its FEN;
    /// `None` once every position has its value or is being searched.
    pub fn next_position(&mut self) -> Option<(usize, String)> {
        let index = (0..self.evals.len())
            .find(|index| self.evals[*index].is_none() && !self.searching.contains(index))?;
        self.searching.push(index);
        Some((index, self.game.positions()[index].to_string()))
    }

    /// How many positions from the start have their value: the review is known that far.
    pub fn known_from_start(&self) -> usize {
        self.evals.iter().take_while(|eval| eval.is_some()).count()
    }

    /// The engine's lines for the position at `index`, any order.
    pub fn store(&mut self, index: usize, lines: &[SearchInfo]) {
        if !self.searching.contains(&index) {
            return;
        }
        self.searching.retain(|searching| *searching != index);
        let side_to_move = self.game.positions()[index].side_to_move();
        let mut lines: Vec<&SearchInfo> = lines.iter().collect();
        lines.sort_by_key(|info| info.multipv);
        let mut stored: Vec<EngineLine> = lines
            .into_iter()
            .map(|info| EngineLine {
                first: info.pv.first().copied(),
                eval: Eval::from_uci(info.score, side_to_move),
            })
            .collect();
        if stored.is_empty() {
            // An engine that said nothing leaves the position even rather than stuck.
            stored.push(EngineLine {
                first: None,
                eval: Eval::Centipawns(0),
            });
        }
        self.evals[index] = Some(PositionEval { lines: stored });
    }

    /// Positions with their value, and how many there are.
    pub fn progress(&self) -> (usize, usize) {
        let done = self.evals.iter().filter(|eval| eval.is_some()).count();
        (done, self.evals.len())
    }

    /// The review, once every position has its value.
    pub fn finish(&self) -> Option<GameReview> {
        let evals: Vec<&PositionEval> = self
            .evals
            .iter()
            .map(Option::as_ref)
            .collect::<Option<_>>()?;
        Some(review(&self.game, &evals))
    }
}

/// A finished position's value is its result; any other still needs the engine.
fn finished_eval(position: &Position) -> Option<PositionEval> {
    let eval = match position.status() {
        GameStatus::Ongoing => return None,
        GameStatus::Checkmate { winner } => Eval::Mate { winner, moves: 0 },
        GameStatus::Stalemate => Eval::Centipawns(0),
    };
    Some(PositionEval {
        lines: vec![EngineLine { first: None, eval }],
    })
}

/// Every move classified in order, then each side's accuracy.
fn review(game: &Game, evals: &[&PositionEval]) -> GameReview {
    let positions = game.positions();
    let mut moves: Vec<MoveReview> = Vec::new();
    let mut facts_so_far: Vec<MoveFacts> = Vec::new();
    let mut in_book = true;
    for (index, &mv) in game.moves().iter().enumerate() {
        let before = &positions[index];
        let mover = before.side_to_move();
        in_book = in_book && index < BOOK_PLIES && is_book(&positions[index + 1]);
        let best_line = evals[index].lines[0];
        let mut facts = MoveFacts {
            legal_moves: before.legal_moves().len(),
            book: in_book,
            played_best: best_line.first == Some(mv),
            best_points: points(evals[index], 0, mover).unwrap_or(0.5),
            second_points: points(evals[index], 1, mover),
            played_points: points(evals[index + 1], 0, mover).unwrap_or(0.5),
            sacrifice: false,
            after_opponent_error: index
                .checked_sub(1)
                .and_then(|previous| facts_so_far.get(previous))
                .is_some_and(MoveFacts::is_error),
        };
        facts.sacrifice = nearly_best(&facts) && is_sacrifice(before, mv);
        let class = classify(&facts);
        moves.push(MoveReview {
            mover,
            class,
            best: best_line.first,
            accuracy: accuracy_of(class, &facts),
        });
        facts_so_far.push(facts);
    }
    let accuracy = [Side::White, Side::Black].map(|side| {
        let of_side: Vec<f64> = moves
            .iter()
            .filter(|reviewed| reviewed.mover == side)
            .map(|reviewed| reviewed.accuracy)
            .collect();
        side_accuracy(&of_side)
    });
    let ratings = [0, 1].map(|index| {
        let side = [Side::White, Side::Black][index];
        (own_moves(&moves, side) >= FEWEST_MOVES).then(|| estimated_rating(accuracy[index]))
    });
    let shares = evals
        .iter()
        .map(|eval| eval.lines[0].eval.white_share())
        .collect();
    GameReview {
        moves,
        accuracy,
        ratings,
        shares,
    }
}

/// The moves `side` chose for itself: not Book, which follows theory, and not Forced.
fn own_moves(moves: &[MoveReview], side: Side) -> usize {
    moves
        .iter()
        .filter(|reviewed| {
            reviewed.mover == side && !matches!(reviewed.class, MoveClass::Book | MoveClass::Forced)
        })
        .count()
}

/// The expected points of the engine's `line` at a position, for `side`.
fn points(eval: &PositionEval, line: usize, side: Side) -> Option<f64> {
    eval.lines
        .get(line)
        .map(|line| line.eval.expected_points(side))
}

fn nearly_best(facts: &MoveFacts) -> bool {
    facts.loss() <= NEARLY_BEST
}

/// Book and forced moves cost nothing; any other keeps what its points lost allow.
fn accuracy_of(class: MoveClass, facts: &MoveFacts) -> f64 {
    match class {
        MoveClass::Book | MoveClass::Forced => 100.0,
        MoveClass::Brilliant
        | MoveClass::Great
        | MoveClass::Best
        | MoveClass::Excellent
        | MoveClass::Good
        | MoveClass::Inaccuracy
        | MoveClass::Mistake
        | MoveClass::Miss
        | MoveClass::Blunder => move_accuracy(facts.loss()),
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::{
        Game, Move, Side,
        engine::{SearchInfo, UciScore},
    };

    use super::ReviewRun;
    use crate::{enums::analysis::MoveClass, services::analysis::class_count};

    fn mv(uci: &str) -> Move {
        uci.parse().expect("a move")
    }

    fn line(multipv: usize, cp: i32, first: &str) -> SearchInfo {
        SearchInfo {
            depth: 16,
            multipv,
            score: UciScore::Cp(cp),
            nodes: 0,
            nps: 0,
            pv: vec![mv(first)],
        }
    }

    /// Fool's mate: f3 e5 g4 Qh4#.
    fn fools_mate() -> Game {
        let mut game = Game::new();
        for uci in ["f2f3", "e7e5", "g2g4", "d8h4"] {
            game.play(mv(uci)).expect("legal");
        }
        game
    }

    #[test]
    fn positions_are_searched_in_order_and_the_mate_needs_no_search() {
        let mut run = ReviewRun::new(&fools_mate());
        assert_eq!(run.progress(), (1, 5), "the final mate is known");
        let (index, first) = run.next_position().expect("the start");
        assert_eq!(index, 0);
        assert!(first.starts_with("rnbqkbnr/pppppppp"));
        let (second, _) = run.next_position().expect("another engine takes the next");
        assert_eq!(second, 1);
        run.store(second, &[line(1, 40, "e7e5")]);
        assert_eq!(
            run.known_from_start(),
            0,
            "the start is still being searched"
        );
        run.store(index, &[line(1, 30, "e2e4")]);
        assert_eq!(run.progress(), (3, 5));
        assert_eq!(run.known_from_start(), 2);
        assert!(run.finish().is_none(), "not done yet");
    }

    #[test]
    fn a_whole_game_reviews_with_its_blunder() {
        let mut run = ReviewRun::new(&fools_mate());
        // White to move: even; Black to move after f3: Black slightly better; then g4 hands
        // Black a mate in one.
        let answers = [
            vec![line(1, 30, "e2e4"), line(2, 25, "d2d4")],
            vec![line(1, 40, "e7e5"), line(2, 35, "d7d5")],
            vec![line(1, -60, "e2e4"), line(2, -70, "b1c3")],
        ];
        for answer in answers {
            let (index, _) = run.next_position().expect("a position to search");
            run.store(index, &answer);
        }
        let (index, _) = run.next_position().expect("the position after g4");
        run.store(
            index,
            &[SearchInfo {
                depth: 16,
                multipv: 1,
                score: UciScore::Mate(1),
                nodes: 0,
                nps: 0,
                pv: vec![mv("d8h4")],
            }],
        );
        assert!(run.next_position().is_none());
        let review = run.finish().expect("done");
        assert_eq!(review.moves[2].class, MoveClass::Blunder, "g4");
        assert_eq!(review.moves[3].class, MoveClass::Best, "Qh4#");
        assert_eq!(class_count(&review, Side::White, MoveClass::Blunder), 1);
        assert!(
            review.accuracy[1] > review.accuracy[0],
            "Black played better"
        );
        assert!((0.0..=100.0).contains(&review.accuracy[0]));
        assert_eq!(
            review.ratings,
            [None, None],
            "two moves each say nothing of a rating"
        );
    }

    #[test]
    fn every_position_is_searched_from_the_first_so_the_graph_has_the_whole_game() {
        let mut game = Game::new();
        for uci in ["e2e4", "e7e5", "g1f3", "d8h4"] {
            game.play(mv(uci)).expect("legal");
        }
        let mut run = ReviewRun::new(&game);
        let (first, _) = run.next_position().expect("a position to search");
        assert_eq!(first, 0, "the opening is searched too, from the start");
        assert_eq!(run.known_from_start(), 0);
    }
}
