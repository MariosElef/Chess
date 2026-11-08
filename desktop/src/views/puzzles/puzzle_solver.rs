use std::time::{Duration, Instant};

use chess_domain::Move;
use gpui_kit::*;

use crate::{
    enums::{
        puzzles::{PuzzleState, PuzzleVerdict},
        sound::Sound,
    },
    models::{
        board::{MoveMade, ShownPosition},
        puzzles::Puzzle,
    },
    services::puzzles::{PuzzleRun, status_line},
    views::{
        board::BoardView,
        sound::{play_move_sound, play_sound},
    },
};

/// How long the opponent seems to think before its scripted answer, so the move is seen.
pub const ANSWER_DELAY: Duration = Duration::from_millis(400);

/// One puzzle on the shared board for Practice and Rush: moves are judged and the opponent answers.
pub struct PuzzleSolver {
    run: Option<PuzzleRun>,
    /// The player's last move, for the status line.
    last_verdict: Option<PuzzleVerdict>,
    /// When the puzzle appeared, for how long it took.
    loaded_at: Option<Instant>,
    /// When it was solved: the time taken stops there.
    solved_at: Option<Instant>,
    board: Entity<BoardView>,
    _board_moves: Subscription,
    /// The opponent's answer on its way; dropped with the puzzle.
    _answer: Option<Task<()>>,
}

impl EventEmitter<PuzzleVerdict> for PuzzleSolver {}

impl PuzzleSolver {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let solver = cx.weak_entity();
        let board = cx.new(|board| {
            BoardView::new(
                move |cx| {
                    solver
                        .upgrade()
                        .map(|solver| solver.read(cx).shown_position())
                },
                board,
            )
        });
        let _board_moves = cx.subscribe(&board, |solver, _, MoveMade(mv), cx| {
            solver.play(*mv, cx);
        });
        Self {
            run: None,
            last_verdict: None,
            loaded_at: None,
            solved_at: None,
            board,
            _board_moves,
            _answer: None,
        }
    }

    /// Sets `puzzle` up with the player's side at the bottom.
    pub fn load(&mut self, puzzle: Puzzle, cx: &mut Context<Self>) {
        let run = PuzzleRun::new(puzzle);
        let solver = run.solver();
        self.run = Some(run);
        self.last_verdict = None;
        self.loaded_at = Some(Instant::now());
        self.solved_at = None;
        self._answer = None;
        self.board.update(cx, |board, cx| {
            board.reset(cx);
            board.face(solver, cx);
        });
        cx.notify();
    }

    /// No puzzle: the board back to the starting position, nothing to move, between puzzles.
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.run = None;
        self.last_verdict = None;
        self.loaded_at = None;
        self.solved_at = None;
        self._answer = None;
        self.board.update(cx, |board, cx| board.reset(cx));
        cx.notify();
    }

    pub fn run(&self) -> Option<&PuzzleRun> {
        self.run.as_ref()
    }

    /// Milliseconds from the puzzle appearing to now, or to its solving, for the server's attempt.
    pub fn elapsed_ms(&self) -> i32 {
        let until = self.solved_at.unwrap_or_else(Instant::now);
        self.loaded_at.map_or(0, |loaded| {
            i32::try_from(until.saturating_duration_since(loaded).as_millis()).unwrap_or(i32::MAX)
        })
    }

    /// What to say about the puzzle: whose move, how the last one went, or solved.
    pub fn status(&self) -> Option<(String, bool)> {
        self.run
            .as_ref()
            .map(|run| status_line(run, self.last_verdict))
    }

    #[cfg(test)]
    pub fn board(&self) -> &Entity<BoardView> {
        &self.board
    }

    /// The player's move: a wrong one is refused with a sound; a right one is played and answered.
    fn play(&mut self, mv: Move, cx: &mut Context<Self>) {
        let Some(run) = self.run.as_mut() else {
            return;
        };
        let before = *run.position();
        let verdict = run.play(mv);
        self.last_verdict = Some(verdict);
        match verdict {
            PuzzleVerdict::Wrong => {
                play_sound(Sound::Illegal, cx);
                self.board.update(cx, |board, cx| {
                    board.drop_all_premoves();
                    board.put_down(cx);
                });
            }
            PuzzleVerdict::Correct => {
                play_move_sound(&before, run.position(), mv, Sound::MoveSelf, false, cx);
                self.answer_soon(cx);
            }
            PuzzleVerdict::Solved => {
                self.solved_at = Some(Instant::now());
                play_move_sound(&before, run.position(), mv, Sound::MoveSelf, false, cx);
                self.board.update(cx, |board, _| board.drop_all_premoves());
            }
        }
        cx.emit(verdict);
        cx.notify();
    }

    fn answer_soon(&mut self, cx: &mut Context<Self>) {
        self._answer = Some(cx.spawn(async move |solver, cx| {
            cx.background_executor().timer(ANSWER_DELAY).await;
            // The solver outlives its answer unless the window is closing.
            let _ = solver.update(cx, |solver, cx| solver.play_answer(cx));
        }));
    }

    /// The opponent's answer; then the player's queued premove, if any, is played and judged.
    fn play_answer(&mut self, cx: &mut Context<Self>) {
        let Some(run) = self.run.as_mut() else {
            return;
        };
        let before = *run.position();
        if let Some(answer) = run.play_answer() {
            play_move_sound(
                &before,
                run.position(),
                answer,
                Sound::MoveOpponent,
                false,
                cx,
            );
        }
        let position = *run.position();
        let premove = self
            .board
            .update(cx, |board, _| board.next_premove(&position));
        match premove {
            Some(mv) => self.play(mv, cx),
            None => cx.notify(),
        }
    }

    /// The puzzle's position, taking only the player's pieces: moved on their turn or premoved.
    fn shown_position(&self) -> ShownPosition {
        match &self.run {
            Some(run) => ShownPosition {
                position: *run.position(),
                last_move: run.last_move(),
                can_move: run.state() != PuzzleState::Solved,
                premoves: true,
                player: Some(run.solver()),
                engine_arrow: None,
                badge: None,
                best_arrow: None,
                outcome: None,
            },
            None => ShownPosition::waiting(),
        }
    }
}

impl Render for PuzzleSolver {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().h_full().child(self.board.clone())
    }
}
