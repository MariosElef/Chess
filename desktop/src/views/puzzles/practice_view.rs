use std::{rc::Rc, time::Duration};

use gpui_kit::{component::slider::SliderState, *};

use super::{
    puzzle_request::{TokenSource, report_attempt, request_puzzle},
    puzzle_solver::PuzzleSolver,
};
use crate::{
    clients::server::ServerClient,
    components::{
        board::{BoardColumn, board_page},
        panel::{OnBack, SidePanel},
        puzzles::{PuzzlePanel, RangePicker},
    },
    enums::puzzles::{PuzzleState, PuzzleVerdict},
    models::{
        puzzles::{AttemptReport, Puzzle, PuzzleRating, RatingRange},
        server::ServerError,
    },
    services::{puzzles::rating_with_change, server::error_message},
    views::ticker::start_ticker,
};

/// How often the puzzle's timer is redrawn.
const TICK: Duration = Duration::from_millis(500);

/// One puzzle at a time in the player's difficulty range; the first try counts for the rating.
pub struct PracticeView {
    client: ServerClient,
    token: TokenSource,
    solver: Entity<PuzzleSolver>,
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
}

impl PracticeView {
    pub fn new(
        client: ServerClient,
        token: TokenSource,
        (slider, range): (Entity<SliderState>, RatingRange),
        cx: &mut Context<Self>,
    ) -> Self {
        let solver = cx.new(PuzzleSolver::new);
        let _verdicts = cx.subscribe(&solver, |view, _, verdict: &PuzzleVerdict, cx| {
            view.judged(*verdict, cx);
        });
        Self {
            client,
            token,
            solver,
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
        }
    }

    /// From the menu: a puzzle at `range`, with the player's rating as the menu knows it.
    pub fn start(
        &mut self,
        range: RatingRange,
        rating: Option<PuzzleRating>,
        cx: &mut Context<Self>,
    ) {
        self.range = range;
        self.player_rating = rating;
        self.next_puzzle(cx);
    }

    /// The slider moved: the label follows; letting go fetches a puzzle at the new difficulty.
    pub fn range_changed(&mut self, range: RatingRange, released: bool, cx: &mut Context<Self>) {
        self.range = range;
        if released {
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
        match request_puzzle(&self.client, &self.token, self.range, cx) {
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
    fn judged(&mut self, verdict: PuzzleVerdict, cx: &mut Context<Self>) {
        let first_try = match verdict {
            PuzzleVerdict::Wrong => Some(false),
            PuzzleVerdict::Solved => Some(true),
            PuzzleVerdict::Correct => None,
        };
        if let (Some(solved), false) = (first_try, self.reported) {
            self.reported = true;
            self.report(solved, cx);
        }
        cx.notify();
    }

    fn report(&mut self, solved: bool, cx: &mut Context<Self>) {
        let solver = self.solver.read(cx);
        let Some(puzzle_id) = solver.run().map(|run| run.puzzle().id.clone()) else {
            return;
        };
        let report = AttemptReport {
            solved,
            rated: true,
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
            .player_rating(self.player_rating.map(rating_with_change))
            .difficulty(RangePicker::new(self.slider.clone(), self.range))
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
            SidePanel::new().child(self.panel(cx)),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui_kit::{
        AppContext as _, Entity, TestAppContext, VisualTestContext, component::slider::SliderState,
    };

    use super::PracticeView;
    use crate::{
        enums::puzzles::PuzzleState,
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
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            let slider = cx.new(|_| SliderState::new());
            let token = Rc::new(move |_: &gpui_kit::App| token.map(str::to_owned));
            PracticeView::new(puzzle_server(), token, (slider, RatingRange::DEFAULT), cx)
        });
        view.update(cx, |view, cx| view.start(RatingRange::DEFAULT, None, cx));
        (view, cx)
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
