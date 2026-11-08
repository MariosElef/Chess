use chess_domain::{GameStatus, Move, Position, Side};

use crate::{
    enums::puzzles::{PuzzleState, PuzzleVerdict},
    models::puzzles::Puzzle,
};

/// One attempt at a puzzle: where the board is, how far into the solution the player got, and
/// whether a wrong move was tried on the way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PuzzleRun {
    puzzle: Puzzle,
    position: Position,
    last_move: Option<Move>,
    /// The solution's next move: the player's at even places, the opponent's at odd.
    next: usize,
    state: PuzzleState,
    mistakes: u32,
}

impl PuzzleRun {
    pub fn new(puzzle: Puzzle) -> Self {
        Self {
            position: puzzle.position,
            last_move: puzzle.last_move,
            puzzle,
            next: 0,
            state: PuzzleState::Solving,
            mistakes: 0,
        }
    }

    pub fn puzzle(&self) -> &Puzzle {
        &self.puzzle
    }

    pub fn position(&self) -> &Position {
        &self.position
    }

    pub fn last_move(&self) -> Option<Move> {
        self.last_move
    }

    pub fn state(&self) -> PuzzleState {
        self.state
    }

    /// The side the player plays: the one to move when the puzzle starts.
    pub fn solver(&self) -> Side {
        self.puzzle.position.side_to_move()
    }

    /// Wrong moves tried so far; a puzzle solved with none was solved clean.
    pub fn mistakes(&self) -> u32 {
        self.mistakes
    }

    /// The player's move: the solution's move, or any checkmate, is played; anything else
    /// leaves the board as it was.
    pub fn play(&mut self, mv: Move) -> PuzzleVerdict {
        let expected = self.puzzle.solution.get(self.next).copied();
        let after = self
            .position
            .play(mv)
            .ok()
            .filter(|after| Some(mv) == expected || is_mate(after));
        let (PuzzleState::Solving, Some(after)) = (self.state, after) else {
            self.mistakes += 1;
            return PuzzleVerdict::Wrong;
        };
        self.advance(after, mv);
        if is_mate(&after) || self.next >= self.puzzle.solution.len() {
            self.state = PuzzleState::Solved;
            PuzzleVerdict::Solved
        } else {
            self.state = PuzzleState::Answering;
            PuzzleVerdict::Correct
        }
    }

    /// The opponent's scripted answer, after a correct move; the player's turn again.
    pub fn play_answer(&mut self) -> Option<Move> {
        if self.state != PuzzleState::Answering {
            return None;
        }
        let answer = *self.puzzle.solution.get(self.next)?;
        let after = self.position.play(answer).ok()?;
        self.advance(after, answer);
        self.state = PuzzleState::Solving;
        Some(answer)
    }

    fn advance(&mut self, after: Position, mv: Move) {
        self.position = after;
        self.last_move = Some(mv);
        self.next += 1;
    }
}

fn is_mate(position: &Position) -> bool {
    matches!(position.status(), GameStatus::Checkmate { .. })
}

#[cfg(test)]
mod tests {
    use chess_domain::{Move, Side};

    use super::PuzzleRun;
    use crate::{
        enums::puzzles::{PuzzleState, PuzzleVerdict},
        models::puzzles::Puzzle,
    };

    fn mv(uci: &str) -> Move {
        uci.parse().expect("a UCI move")
    }

    /// Lichess puzzle 5laTS: a queen sacrifice and mate in three for White.
    fn puzzle() -> Puzzle {
        Puzzle {
            id: "5laTS".to_owned(),
            rating: 1593,
            plays: 2015,
            themes: vec!["mateIn3".to_owned()],
            position: "7k/pp2r1p1/2pb3r/3p1B1Q/3N4/8/P2q2PP/4R2K w - - 1 1"
                .parse()
                .expect("valid"),
            last_move: Some(mv("h7h8")),
            solution: ["h5e8", "e7e8", "e1e8", "d6f8", "e8f8"].map(mv).to_vec(),
        }
    }

    #[test]
    fn the_solution_is_played_with_the_opponents_answers_between() {
        let mut run = PuzzleRun::new(puzzle());
        assert_eq!(run.solver(), Side::White);
        assert_eq!(run.play(mv("h5e8")), PuzzleVerdict::Correct);
        assert_eq!(run.state(), PuzzleState::Answering);
        assert_eq!(run.play_answer(), Some(mv("e7e8")));
        assert_eq!(run.play(mv("e1e8")), PuzzleVerdict::Correct);
        assert_eq!(run.play_answer(), Some(mv("d6f8")));
        assert_eq!(run.play(mv("e8f8")), PuzzleVerdict::Solved);
        assert_eq!(run.state(), PuzzleState::Solved);
        assert_eq!(run.mistakes(), 0);
    }

    #[test]
    fn a_wrong_move_changes_nothing_but_counts() {
        let mut run = PuzzleRun::new(puzzle());
        let before = *run.position();
        assert_eq!(run.play(mv("h5g6")), PuzzleVerdict::Wrong);
        assert_eq!(*run.position(), before);
        assert_eq!(run.state(), PuzzleState::Solving);
        assert_eq!(run.mistakes(), 1);
        assert_eq!(run.play(mv("h5e8")), PuzzleVerdict::Correct);
    }

    #[test]
    fn no_move_is_taken_while_the_opponent_answers() {
        let mut run = PuzzleRun::new(puzzle());
        run.play(mv("h5e8"));
        assert_eq!(run.play(mv("e1e8")), PuzzleVerdict::Wrong);
        assert_eq!(run.play_answer(), Some(mv("e7e8")));
    }

    #[test]
    fn any_checkmate_solves_the_puzzle() {
        let mut run = PuzzleRun::new(Puzzle {
            position: "6k1/5ppp/8/8/8/8/8/R3R1K1 w - - 0 1"
                .parse()
                .expect("valid"),
            solution: vec![mv("a1a8")],
            ..puzzle()
        });
        assert_eq!(
            run.play(mv("e1e8")),
            PuzzleVerdict::Solved,
            "another rook mates too"
        );
    }
}
