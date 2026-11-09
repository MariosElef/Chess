use std::{rc::Rc, time::Duration};

use gpui_kit::{
    component::{slider::SliderState, v_flex},
    *,
};

use super::{
    puzzle_request::{TokenSource, report_attempt, request_puzzle, request_rated_puzzle},
    puzzle_solver::PuzzleSolver,
};
use crate::{
    clients::server::ServerClient,
    components::{
        board::{BoardColumn, board_page},
        panel::{OnBack, SidePanel},
        puzzles::{PlayerHeader, PuzzlePanel, RangePicker, SessionResults},
    },
    enums::puzzles::{PracticeKind, PuzzleState, PuzzleVerdict},
    models::{
        puzzles::{AttemptReport, Puzzle, PuzzleOutcome, PuzzleRating, RatingRange},
        server::ServerError,
    },
    services::{puzzles::rating_with_change, server::error_message},
    views::{
        live::{SessionSource, my_face},
        ticker::start_ticker,
    },
};

/// How often the puzzle's timer is redrawn.
const TICK: Duration = Duration::from_millis(500);
/// How long a solved puzzle stays on the board before the next one comes by itself.
pub const SOLVED_PAUSE: Duration = Duration::from_millis(1500);

/// One puzzle at a time: rated ones at the player's own rating, whose first try moves it, or
/// custom ones in a difficulty range the player picks, which move nothing.
pub struct PracticeView {
    client: ServerClient,
    token: TokenSource,
    solver: Entity<PuzzleSolver>,
    kind: PracticeKind,
    range: RatingRange,
    /// The difficulty slider, shared with the menu.
    slider: Entity<SliderState>,
    loading: bool,
    problem: Option<String>,
    /// The rating after the last rated attempt, with its change.
    player_rating: Option<PuzzleRating>,
    /// This puzzle's first try has been sent.
    reported: bool,
    _verdicts: Subscription,
    /// Back to the menu, from the top of the side column.
    on_back: OnBack,
    /// Redraws the puzzle's timer while it is being solved; it stops when the puzzle is.
    _ticker: Option<Task<()>>,
    /// The next puzzle on its way; dropped when Practice is left.
    _request: Option<Task<()>>,
    /// The pause after a solve, before the next puzzle is asked for; dropped when Practice is
    /// left or Next is pressed first.
    _advance: Option<Task<()>>,
    /// Each puzzle of this session as its first try went, for the row of ticks and crosses.
    outcomes: Vec<PuzzleOutcome>,
    /// Who is signed in, for their picture over their rating.
    session: Option<SessionSource>,
}

impl PracticeView {
    pub fn new(
        client: ServerClient,
        token: TokenSource,
        (slider, range): (Entity<SliderState>, RatingRange),
        cx: &mut Context<Self>,
    ) -> Self {
        let solver = cx.new(PuzzleSolver::new);
        // A wrong move is taken back after its miss, to try the puzzle again.
        solver.update(cx, |solver, _| solver.set_retries(true));
        let _verdicts = cx.subscribe(&solver, |view, _, verdict: &PuzzleVerdict, cx| {
            view.judged(*verdict, cx);
        });
        Self {
            client,
            token,
            solver,
            kind: PracticeKind::Rated,
            range,
            slider,
            loading: false,
            problem: None,
            player_rating: None,
            reported: false,
            _verdicts,
            on_back: Rc::new(|_, _| {}),
            _ticker: None,
            _request: None,
            _advance: None,
            outcomes: Vec::new(),
            session: None,
        }
    }

    /// The signed-in player, whose picture shows over their rating.
    pub fn face_from(&mut self, session: SessionSource) {
        self.session = Some(session);
    }

    /// From the menu: rated puzzles, or custom ones at `range`, with the player's rating as the
    /// menu knows it.
    pub fn start(
        &mut self,
        (kind, range): (PracticeKind, RatingRange),
        rating: Option<PuzzleRating>,
        cx: &mut Context<Self>,
    ) {
        self.kind = kind;
        self.range = range;
        self.outcomes.clear();
        self.player_rating = rating;
        self.next_puzzle(cx);
    }

    /// The slider moved: the label follows; letting go fetches a custom puzzle at the new
    /// difficulty. Rated puzzles have no slider, so it changes nothing for them.
    pub fn range_changed(&mut self, range: RatingRange, released: bool, cx: &mut Context<Self>) {
        self.range = range;
        if released && self.kind == PracticeKind::Custom {
            self.next_puzzle(cx);
        }
        cx.notify();
    }

    #[cfg(test)]
    pub fn solver(&self) -> &Entity<PuzzleSolver> {
        &self.solver
    }

    #[cfg(test)]
    pub fn problem(&self) -> Option<&str> {
        self.problem.as_deref()
    }

    #[cfg(test)]
    pub fn player_rating(&self) -> Option<PuzzleRating> {
        self.player_rating
    }

    fn next_puzzle(&mut self, cx: &mut Context<Self>) {
        self._advance = None;
        match self.kind {
            PracticeKind::Rated => {
                let request = request_rated_puzzle(&self.client, &self.token, cx);
                self.fetch(request, cx);
            }
            PracticeKind::Custom => {
                let request = request_puzzle(&self.client, &self.token, self.range, cx);
                self.fetch(request, cx);
            }
        }
    }

    /// Waits for the puzzle asked for; asking while signed out shows why nothing came.
    fn fetch(
        &mut self,
        request: Result<impl Future<Output = Result<Puzzle, ServerError>> + 'static, &'static str>,
        cx: &mut Context<Self>,
    ) {
        match request {
            Ok(request) => {
                self.loading = true;
                self.problem = None;
                self._request = Some(cx.spawn(async move |view, cx| {
                    let result = request.await;
                    let _ = view.update(cx, |view, cx| view.loaded(result, cx));
                }));
            }
            Err(reason) => self.problem = Some(reason.to_owned()),
        }
        cx.notify();
    }

    fn loaded(&mut self, result: Result<Puzzle, ServerError>, cx: &mut Context<Self>) {
        self.loading = false;
        match result {
            Ok(puzzle) => {
                self.reported = false;
                self.solver.update(cx, |solver, cx| solver.load(puzzle, cx));
                self._ticker = Some(start_ticker(TICK, cx, PracticeView::tick));
            }
            Err(error) => self.problem = Some(error_message(&error)),
        }
        cx.notify();
    }

    /// The first try decides: a wrong move loses, solving cleanly wins; later moves change nothing.
    /// A solved puzzle, first try or not, moves on to the next by itself after a moment.
    fn judged(&mut self, verdict: PuzzleVerdict, cx: &mut Context<Self>) {
        if verdict == PuzzleVerdict::Solved {
            self.advance_soon(cx);
        }
        let first_try = match verdict {
            PuzzleVerdict::Wrong => Some(false),
            PuzzleVerdict::Solved => Some(true),
            PuzzleVerdict::Correct => None,
        };
        if let (Some(solved), false) = (first_try, self.reported) {
            self.reported = true;
            self.record_outcome(solved, cx);
            self.report(solved, cx);
        }
        cx.notify();
    }

    /// The puzzle's first try, for the row under the player's number.
    fn record_outcome(&mut self, solved: bool, cx: &App) {
        let rating = self.solver.read(cx).run().map(|run| run.puzzle().rating);
        if let Some(rating) = rating {
            self.outcomes.push(PuzzleOutcome { solved, rating });
        }
    }

    /// The player's picture over their rating, rated, or over how many they solved, custom; then
    /// this session's puzzles.
    fn scoreboard(&self, cx: &App) -> Div {
        let face = self
            .session
            .as_ref()
            .and_then(|session| my_face(session, cx));
        let number = match self.kind {
            PracticeKind::Rated => self
                .player_rating
                .map_or_else(|| "–".to_owned(), rating_with_change),
            PracticeKind::Custom => solved_count(&self.outcomes),
        };
        v_flex()
            .gap_4()
            .child(PlayerHeader::new(face, number))
            .child(SessionResults::new(self.outcomes.clone()))
    }

    /// The next puzzle after the solved one has been seen for a moment.
    fn advance_soon(&mut self, cx: &mut Context<Self>) {
        self._advance = Some(cx.spawn(async move |view, cx| {
            cx.background_executor().timer(SOLVED_PAUSE).await;
            // Practice outlives its pause unless the window is closing.
            let _ = view.update(cx, |view, cx| view.next_puzzle(cx));
        }));
    }

    fn report(&mut self, solved: bool, cx: &mut Context<Self>) {
        let solver = self.solver.read(cx);
        let Some(puzzle_id) = solver.run().map(|run| run.puzzle().id.clone()) else {
            return;
        };
        let report = AttemptReport {
            solved,
            rated: self.kind == PracticeKind::Rated,
            duration_ms: solver.elapsed_ms(),
        };
        let Some(request) = report_attempt(&self.client, &self.token, &puzzle_id, report, cx)
        else {
            return;
        };
        cx.spawn(async move |view, cx| {
            if let Ok(Some(rating)) = request.await {
                let _ = view.update(cx, |view, cx| {
                    view.player_rating = Some(rating);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Redraws the timer while the puzzle is being solved; false once it is.
    fn tick(&mut self, cx: &mut Context<Self>) -> bool {
        let solving = self
            .solver
            .read(cx)
            .run()
            .is_some_and(|run| run.state() != PuzzleState::Solved);
        if solving {
            cx.notify();
        }
        solving
    }

    fn panel(&self, cx: &mut Context<Self>) -> PuzzlePanel {
        let solver = self.solver.read(cx);
        let status = match solver.status() {
            Some(status) => status,
            None if self.loading => ("Finding a puzzle…".to_owned(), false),
            None => (String::new(), false),
        };
        let run = solver.run();
        let puzzle = run.map(|run| run.puzzle());
        let elapsed = run.map(|_| Duration::from_millis(solver.elapsed_ms().unsigned_abs().into()));
        let view = cx.entity().downgrade();
        let on_back = Rc::clone(&self.on_back);
        PuzzlePanel::new(status, run.map(|run| run.solver()))
            .puzzle(
                puzzle.map(|puzzle| puzzle.rating),
                puzzle
                    .map(|puzzle| puzzle.themes.clone())
                    .unwrap_or_default(),
            )
            .elapsed(elapsed)
            .problem(self.problem.clone())
            // Rated, the rating shows large over the session instead.
            .player_rating(
                self.player_rating
                    .filter(|_| self.kind == PracticeKind::Custom)
                    .map(rating_with_change),
            )
            .difficulty(
                (self.kind == PracticeKind::Custom)
                    .then(|| RangePicker::new(self.slider.clone(), self.range)),
            )
            .loading(self.loading)
            .on_next(move |_, cx| {
                let _ = view.update(cx, |view, cx| view.next_puzzle(cx));
            })
            .on_back(move |window, cx| on_back(window, cx))
    }
}

impl PracticeView {
    /// Leaving Practice stops its timer and drops its puzzles, so coming back finds a new one.
    pub fn leave(&mut self, cx: &mut Context<Self>) {
        self.solver.update(cx, |solver, cx| solver.clear(cx));
        self._ticker = None;
        self._request = None;
        self._advance = None;
        self.loading = false;
    }

    pub fn on_back(&mut self, on_back: impl Fn(&mut Window, &mut App) + 'static) {
        self.on_back = Rc::new(on_back);
    }
}

impl Render for PracticeView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        board_page(
            BoardColumn::new(self.solver.clone()),
            SidePanel::new()
                .child(self.panel(cx))
                .child(self.scoreboard(cx)),
        )
    }
}

/// How many of the session's puzzles were solved on the first try.
fn solved_count(outcomes: &[PuzzleOutcome]) -> String {
    outcomes
        .iter()
        .filter(|outcome| outcome.solved)
        .count()
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui_kit::{
        AppContext as _, Entity, TestAppContext, VisualTestContext, component::slider::SliderState,
    };

    use super::{PracticeView, SOLVED_PAUSE};
    use crate::views::puzzles::puzzle_solver::MISS_PAUSE;
    use crate::{
        enums::{
            analysis::MoveClass,
            puzzles::{PracticeKind, PuzzleState},
        },
        models::puzzles::RatingRange,
        testing::{ANSWER_WAIT, click, drag, play_and_wait, puzzle_server, wait_until},
    };

    fn state(view: &Entity<PracticeView>, cx: &mut VisualTestContext) -> Option<PuzzleState> {
        view.update(cx, |view, cx| {
            view.solver().read(cx).run().map(|run| run.state())
        })
    }

    fn opened<'a>(
        cx: &'a mut TestAppContext,
        token: Option<&'static str>,
    ) -> (Entity<PracticeView>, &'a mut VisualTestContext) {
        opened_as(cx, token, PracticeKind::Rated)
    }

    fn opened_as<'a>(
        cx: &'a mut TestAppContext,
        token: Option<&'static str>,
        kind: PracticeKind,
    ) -> (Entity<PracticeView>, &'a mut VisualTestContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            let slider = cx.new(|_| SliderState::new());
            let token = Rc::new(move |_: &gpui_kit::App| token.map(str::to_owned));
            PracticeView::new(puzzle_server(), token, (slider, RatingRange::DEFAULT), cx)
        });
        view.update(cx, |view, cx| {
            view.start((kind, RatingRange::DEFAULT), None, cx);
        });
        (view, cx)
    }

    #[gpui_kit::test]
    fn a_custom_puzzle_solved_first_try_leaves_the_rating_alone(cx: &mut TestAppContext) {
        let (view, cx) = opened_as(cx, Some("abc"), PracticeKind::Custom);
        wait_until(cx, |cx| state(&view, cx).is_some());
        play_and_wait(cx, "square-h5", "square-e8");
        play_and_wait(cx, "square-e1", "square-e8");
        play_and_wait(cx, "square-e8", "square-f8");
        assert_eq!(state(&view, cx), Some(PuzzleState::Solved));
        std::thread::sleep(ANSWER_WAIT);
        cx.run_until_parked();
        assert_eq!(view.update(cx, |view, _| view.player_rating()), None);
    }

    #[gpui_kit::test]
    fn each_right_move_gets_a_green_tick_on_its_piece(cx: &mut TestAppContext) {
        let (view, cx) = opened(cx, Some("abc"));
        wait_until(cx, |cx| state(&view, cx).is_some());
        let badge =
            |cx: &mut VisualTestContext| view.update(cx, |view, cx| view.solver().read(cx).badge());
        assert_eq!(badge(cx), None, "nothing before the player moves");
        play_and_wait(cx, "square-h5", "square-e8");
        let after_answer = badge(cx);
        assert_eq!(
            after_answer, None,
            "the opponent's answer takes the tick's place"
        );
        play_and_wait(cx, "square-e1", "square-e8");
        play_and_wait(cx, "square-e8", "square-f8");
        assert_eq!(
            badge(cx),
            Some(("f8".parse().expect("a square"), MoveClass::Good)),
            "the solving move keeps its tick"
        );
    }

    #[gpui_kit::test]
    fn a_solved_puzzle_moves_on_to_the_next_by_itself(cx: &mut TestAppContext) {
        let (view, cx) = opened(cx, Some("abc"));
        wait_until(cx, |cx| state(&view, cx).is_some());
        play_and_wait(cx, "square-h5", "square-e8");
        play_and_wait(cx, "square-e1", "square-e8");
        play_and_wait(cx, "square-e8", "square-f8");
        assert_eq!(state(&view, cx), Some(PuzzleState::Solved));
        cx.executor().advance_clock(SOLVED_PAUSE);
        wait_until(cx, |cx| {
            state(&view, cx).is_some_and(|state| state != PuzzleState::Solved)
        });
    }

    #[gpui_kit::test]
    fn a_first_try_solve_is_rated_and_shown(cx: &mut TestAppContext) {
        let (view, cx) = opened(cx, Some("abc"));
        wait_until(cx, |cx| state(&view, cx).is_some());
        play_and_wait(cx, "square-h5", "square-e8");
        play_and_wait(cx, "square-e1", "square-e8");
        play_and_wait(cx, "square-e8", "square-f8");
        assert_eq!(state(&view, cx), Some(PuzzleState::Solved));
        wait_until(cx, |cx| {
            view.update(cx, |view, _| view.player_rating().is_some())
        });
        let rating = view.update(cx, |view, _| view.player_rating());
        assert_eq!(
            rating.map(|rating| (rating.rating, rating.change)),
            Some((1512, Some(12)))
        );
    }

    #[gpui_kit::test]
    fn a_wrong_move_is_tried_again(cx: &mut TestAppContext) {
        let (view, cx) = opened(cx, Some("abc"));
        wait_until(cx, |cx| state(&view, cx).is_some());
        click(cx, "square-h5");
        click(cx, "square-g6");
        let status = view.update(cx, |view, cx| {
            view.solver().read(cx).status().map(|(status, _)| status)
        });
        assert_eq!(status.as_deref(), Some("Not the move. Try again."));
        assert_eq!(state(&view, cx), Some(PuzzleState::Solving));
        let shows_miss = |cx: &mut VisualTestContext| {
            view.update(cx, |view, cx| view.solver().read(cx).shows_miss())
        };
        assert!(
            shows_miss(cx),
            "the wrong move stays a moment with its miss"
        );
        cx.executor().advance_clock(MISS_PAUSE);
        cx.run_until_parked();
        assert!(!shows_miss(cx), "then it is taken back to try again");
    }

    #[gpui_kit::test]
    fn the_opponents_pieces_cannot_be_picked_up(cx: &mut TestAppContext) {
        let (view, cx) = opened(cx, Some("abc"));
        wait_until(cx, |cx| state(&view, cx).is_some());
        click(cx, "square-h6");
        let selected = view.update(cx, |view, cx| {
            view.solver().read(cx).board().read(cx).selected()
        });
        assert_eq!(selected, None, "Black's rook is not the player's to move");
        drag(cx, "square-h6", "square-h1");
        let premoves = view.update(cx, |view, cx| {
            view.solver()
                .read(cx)
                .board()
                .read(cx)
                .premoves()
                .is_empty()
        });
        assert!(
            premoves,
            "the opponent's pieces are never the player's to premove"
        );
    }

    #[gpui_kit::test]
    fn a_move_premoved_during_the_answer_plays_as_it_lands(cx: &mut TestAppContext) {
        let (view, cx) = opened(cx, Some("abc"));
        wait_until(cx, |cx| state(&view, cx).is_some());
        click(cx, "square-h5");
        click(cx, "square-e8");
        assert_eq!(state(&view, cx), Some(PuzzleState::Answering));
        click(cx, "square-e1");
        click(cx, "square-e8");
        let queued = view.update(cx, |view, cx| {
            !view
                .solver()
                .read(cx)
                .board()
                .read(cx)
                .premoves()
                .is_empty()
        });
        assert!(queued, "Rxe8 waits for Black's answer");
        cx.executor().advance_clock(ANSWER_WAIT);
        cx.run_until_parked();
        let moves = view.update(cx, |view, cx| {
            view.solver()
                .read(cx)
                .run()
                .map(|run| run.position().to_string())
        });
        assert!(
            moves.as_ref().is_some_and(|fen| fen.starts_with("4Rb1k/")),
            "the answer Rxe8, the premove Rxe8, then the next answer Bf8: {moves:?}"
        );
        play_and_wait(cx, "square-e8", "square-f8");
        assert_eq!(state(&view, cx), Some(PuzzleState::Solved));
    }

    #[gpui_kit::test]
    fn signed_out_there_is_no_puzzle_to_fetch(cx: &mut TestAppContext) {
        let (view, cx) = opened(cx, None);
        let problem = view.update(cx, |view, _| view.problem().map(str::to_owned));
        assert_eq!(problem.as_deref(), Some("Sign in to solve puzzles."));
    }
}
