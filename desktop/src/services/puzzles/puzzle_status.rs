use super::puzzle_run::PuzzleRun;
use crate::{
    enums::puzzles::{PuzzleState, PuzzleVerdict},
    i18n::tr,
    services::board::side_name,
};

/// What the panel says about the puzzle after the player's move; good news comes back `true`.
pub fn status_line(run: &PuzzleRun, last: Option<PuzzleVerdict>) -> (String, bool) {
    let solver = side_name(run.solver());
    match (run.state(), last) {
        (PuzzleState::Solved, _) if run.mistakes() == 0 => (tr!("puzzles.status.solved"), true),
        (PuzzleState::Solved, _) => (solved_after(run.mistakes()), true),
        (PuzzleState::Answering, _) => (tr!("puzzles.status.correct"), true),
        (PuzzleState::Solving, Some(PuzzleVerdict::Wrong)) => (tr!("puzzles.status.wrong"), false),
        (PuzzleState::Solving, Some(PuzzleVerdict::Correct)) => {
            (tr!("puzzles.status.keep_going", side = solver), true)
        }
        (PuzzleState::Solving, Some(PuzzleVerdict::Solved) | None) => {
            (tr!("puzzles.status.find_best", side = solver), false)
        }
    }
}

/// "Solved, after 1 wrong try", or "after 2 wrong tries".
fn solved_after(mistakes: u32) -> String {
    if mistakes == 1 {
        tr!("puzzles.status.solved_after_one")
    } else {
        tr!("puzzles.status.solved_after_other", count = mistakes)
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::Move;

    use super::status_line;
    use crate::{models::puzzles::Puzzle, services::puzzles::PuzzleRun};

    fn mv(uci: &str) -> Move {
        uci.parse().expect("a UCI move")
    }

    fn run() -> PuzzleRun {
        PuzzleRun::new(Puzzle {
            id: "mate".to_owned(),
            rating: 900,
            plays: 1,
            themes: Vec::new(),
            position: "6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1".parse().expect("valid"),
            last_move: None,
            solution: vec![mv("a1a8")],
        })
    }

    #[test]
    fn the_line_follows_the_puzzle() {
        let mut run = run();
        assert_eq!(status_line(&run, None).0, "Find the best move for White.");
        let wrong = run.play(mv("a1a7"));
        assert_eq!(
            status_line(&run, Some(wrong)),
            ("Not the move. Try again.".to_owned(), false)
        );
        let solved = run.play(mv("a1a8"));
        assert_eq!(
            status_line(&run, Some(solved)).0,
            "Solved, after 1 wrong try"
        );
    }
}
