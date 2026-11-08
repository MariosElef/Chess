use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui_kit::*;

use super::{
    puzzle_request::{TokenSource, report_attempt, request_puzzle},
    puzzle_solver::{MISS_PAUSE, PuzzleSolver},
};
use crate::{
    clients::server::ServerClient,
    components::{
        board::{BoardColumn, board_page},
        panel::{OnBack, PanelTitle, SidePanel},
        puzzles::{RUSH_PICTURE, RushResult, RushScore},
    },
    enums::{
        puzzles::{PuzzleVerdict, RushKind},
        sound::Sound,
    },
    models::{
        puzzles::{AttemptReport, Puzzle, RushRecord},
        server::ServerError,
    },
    services::{
        puzzles::RushRun,
        server::{error_message, is_problem},
    },
    views::{sound::play_sound, ticker::start_ticker},
};

/// How often the rush clock is redrawn and checked.
const TICK: Duration = Duration::from_millis(200);
/// The server's answer when a band has no puzzle left that the player has not seen.
const NO_NEW_PUZZLE: &str = "/problems/no-puzzle-in-range";
/// A solved puzzle stays on the board this long before the next one, so its last move is seen.
const NEXT_DELAY: Duration = Duration::from_millis(300);
/// The count before a rush: from 3, a second a number.
const COUNT_FROM: u8 = 3;
const COUNT_STEP: Duration = Duration::from_secs(1);

/// A rush's score reached the server: the Rush panel's bests are fetched again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RushSaved;

/// Puzzle Rush: ever harder puzzles until three mistakes or the clock end it.
pub struct RushView {
    client: ServerClient,
    token: TokenSource,
    solver: Entity<PuzzleSolver>,
    rush: Option<RushRun>,
    /// When the first puzzle appeared; the clock runs from there.
    started: Option<Instant>,
    over: bool,
    /// The next puzzle, fetched ahead.
    upcoming: Option<Puzzle>,
    /// A fetch is on its way.
    fetching: bool,
    /// The puzzle on its way; a new rush drops the last one's.
    _fetch: Option<Task<()>>,
    /// The board is waiting for the fetch on its way.
    waiting: bool,
    /// The narrow band was empty, so the next fetch asks the wide one.
    widened: bool,
    problem: Option<String>,
    /// The server's answer to the finished rush: the best, and whether this one set it.
    record: Option<RushRecord>,
    _ticker: Option<Task<()>>,
    _next: Option<Task<()>>,
    /// The count before a rush, as chess.com's: 3, 2, 1, then the first puzzle and the clock.
    countdown: Option<u8>,
    _countdown: Option<Task<()>>,
    _verdicts: Subscription,
    /// Back to the Rush panel, from the arrow at the top of the side column.
    on_back: OnBack,
}

impl EventEmitter<RushSaved> for RushView {}

impl RushView {
    pub fn new(client: ServerClient, token: TokenSource, cx: &mut Context<Self>) -> Self {
        let solver = cx.new(PuzzleSolver::new);
        let _verdicts = cx.subscribe(&solver, |view, _, verdict: &PuzzleVerdict, cx| {
            view.judged(*verdict, cx);
        });
        Self {
            client,
            token,
            solver,
            rush: None,
            started: None,
            over: false,
            upcoming: None,
            fetching: false,
            _fetch: None,
            waiting: false,
            widened: false,
            problem: None,
            record: None,
            _ticker: None,
            _next: None,
            countdown: None,
            _countdown: None,
            _verdicts,
            on_back: Rc::new(|_, _| {}),
        }
    }

    #[cfg(test)]
    pub fn rush(&self) -> Option<RushRun> {
        self.rush
    }

    #[cfg(test)]
    pub fn is_over(&self) -> bool {
        self.over
    }

    #[cfg(test)]
    pub fn solver(&self) -> &Entity<PuzzleSolver> {
        &self.solver
    }

    pub fn start(&mut self, kind: RushKind, cx: &mut Context<Self>) {
        self.rush = Some(RushRun::new(kind));
        self.started = None;
        self.over = false;
        self.upcoming = None;
        self.problem = None;
        self.record = None;
        self._ticker = None;
        self.widened = false;
        self.fetching = false;
        self._fetch = None;
        self.solver.update(cx, |solver, cx| solver.clear(cx));
        // The first puzzle is fetched during the count and shown once it ends.
        self.waiting = false;
        self.fetch_next(cx);
        self.count_down(cx);
        cx.notify();
    }

    /// 3, 2, 1 over the board, a beep on each, then the start: the puzzle and its clock.
    fn count_down(&mut self, cx: &mut Context<Self>) {
        self.countdown = Some(COUNT_FROM);
        play_sound(Sound::Notify, cx);
        self._countdown = Some(cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor().timer(COUNT_STEP).await;
                let counting = view
                    .update(cx, |view, cx| view.count_step(cx))
                    .unwrap_or(false);
                if !counting {
                    break;
                }
            }
        }));
    }

    /// One second of the count; false once the rush has begun.
    fn count_step(&mut self, cx: &mut Context<Self>) -> bool {
        let next = self
            .countdown
            .and_then(|count| count.checked_sub(1))
            .filter(|count| *count > 0);
        self.countdown = next;
        cx.notify();
        if next.is_some() {
            play_sound(Sound::Notify, cx);
            return true;
        }
        play_sound(Sound::GameStart, cx);
        match self.upcoming.take() {
            Some(puzzle) => self.show(puzzle, cx),
            None => self.waiting = true,
        }
        false
    }

    fn elapsed(&self) -> Duration {
        self.started
            .map_or(Duration::ZERO, |started| started.elapsed())
    }

    /// Asks for the puzzle after the ones served, unless one is already on its way.
    fn fetch_next(&mut self, cx: &mut Context<Self>) {
        let Some(rush) = self.rush else {
            return;
        };
        if self.fetching || self.upcoming.is_some() {
            return;
        }
        let band = if self.widened {
            rush.next_wide_band()
        } else {
            rush.next_band()
        };
        match request_puzzle(&self.client, &self.token, band, cx) {
            Ok(request) => {
                self.fetching = true;
                self._fetch = Some(cx.spawn(async move |view, cx| {
                    let result = request.await;
                    let _ = view.update(cx, |view, cx| view.fetched(result, cx));
                }));
            }
            Err(reason) => self.problem = Some(reason.to_owned()),
        }
    }

    fn fetched(&mut self, result: Result<Puzzle, ServerError>, cx: &mut Context<Self>) {
        self.fetching = false;
        if self.over {
            return;
        }
        match result {
            Ok(puzzle) => {
                self.widened = false;
                self.problem = None;
                if self.waiting {
                    self.show(puzzle, cx);
                } else {
                    self.upcoming = Some(puzzle);
                }
            }
            Err(error) if !self.widened && is_problem(&error, NO_NEW_PUZZLE) => {
                self.widened = true;
                self.fetch_next(cx);
            }
            Err(error) => self.problem = Some(error_message(&error)),
        }
        cx.notify();
    }

    /// After a failed fetch: ask again, from the narrow band.
    fn retry(&mut self, cx: &mut Context<Self>) {
        self.problem = None;
        self.widened = false;
        self.fetch_next(cx);
        cx.notify();
    }

    /// Puts the puzzle on the board, starts the clock with the first one, and fetches the next.
    fn show(&mut self, puzzle: Puzzle, cx: &mut Context<Self>) {
        self.waiting = false;
        if let Some(rush) = self.rush.as_mut() {
            rush.served_one();
        }
        if self.started.is_none() {
            self.started = Some(Instant::now());
            self._ticker = Some(start_ticker(TICK, cx, RushView::tick));
        }
        self.solver.update(cx, |solver, cx| solver.load(puzzle, cx));
        self.fetch_next(cx);
    }

    /// A solved puzzle scores; a wrong move is a strike; the next follows unless the rush is over.
    fn judged(&mut self, verdict: PuzzleVerdict, cx: &mut Context<Self>) {
        let Some(rush) = self.rush.as_mut() else {
            return;
        };
        match verdict {
            PuzzleVerdict::Correct => {}
            PuzzleVerdict::Solved => {
                rush.solved_one();
                self.report(true, cx);
                self.next_soon(cx);
            }
            PuzzleVerdict::Wrong => {
                rush.missed_one();
                self.report(false, cx);
                // The wrong move shows with its miss for a moment before the next puzzle.
                self.next_after(MISS_PAUSE, cx);
            }
        }
        cx.notify();
    }

    /// A rush puzzle is remembered, so it is not served again, but never rated.
    fn report(&self, solved: bool, cx: &mut Context<Self>) {
        let solver = self.solver.read(cx);
        let Some(puzzle_id) = solver.run().map(|run| run.puzzle().id.clone()) else {
            return;
        };
        let report = AttemptReport {
            solved,
            rated: false,
            duration_ms: solver.elapsed_ms(),
        };
        if let Some(request) = report_attempt(&self.client, &self.token, &puzzle_id, report, cx) {
            cx.spawn(async move |_, _| {
                // Remembering the puzzle is a nicety: a failed report changes nothing here.
                let _ = request.await;
            })
            .detach();
        }
    }

    fn next_soon(&mut self, cx: &mut Context<Self>) {
        self.next_after(NEXT_DELAY, cx);
    }

    fn next_after(&mut self, pause: Duration, cx: &mut Context<Self>) {
        self._next = Some(cx.spawn(async move |view, cx| {
            cx.background_executor().timer(pause).await;
            let _ = view.update(cx, |view, cx| view.advance(cx));
        }));
    }

    fn advance(&mut self, cx: &mut Context<Self>) {
        if self.rush.is_some_and(|rush| rush.is_over(self.elapsed())) {
            self.finish(cx);
            return;
        }
        match self.upcoming.take() {
            Some(puzzle) => self.show(puzzle, cx),
            None => {
                self.waiting = true;
                self.solver.update(cx, |solver, cx| solver.clear(cx));
                self.fetch_next(cx);
            }
        }
        cx.notify();
    }

    /// Redraws the clock and ends a timed rush when it runs out; whether to keep ticking.
    fn tick(&mut self, cx: &mut Context<Self>) -> bool {
        if self.over {
            return false;
        }
        if self.rush.is_some_and(|rush| rush.is_over(self.elapsed())) {
            self.finish(cx);
            return false;
        }
        cx.notify();
        true
    }

    fn finish(&mut self, cx: &mut Context<Self>) {
        self.over = true;
        self._ticker = None;
        self._next = None;
        self.countdown = None;
        self._countdown = None;
        self.upcoming = None;
        self.solver.update(cx, |solver, cx| solver.clear(cx));
        play_sound(Sound::GameEnd, cx);
        self.save_score(cx);
        cx.notify();
    }

    /// Leaving the page mid-rush ends it, and its score counts, as on chess.com.
    pub fn leave(&mut self, cx: &mut Context<Self>) {
        if self.countdown.is_some() {
            self.call_off();
            return;
        }
        if self.rush.is_some() && !self.over {
            self.finish(cx);
        }
    }

    /// Left during the count: the rush never began, so nothing counts and nothing is saved.
    fn call_off(&mut self) {
        self.rush = None;
        self.countdown = None;
        self._countdown = None;
        self.upcoming = None;
        self.fetching = false;
        self._fetch = None;
    }

    fn save_score(&mut self, cx: &mut Context<Self>) {
        let (Some(rush), Some(token)) = (self.rush, (self.token)(cx)) else {
            return;
        };
        let request = self.client.record_rush(&token, rush.kind(), rush.solved());
        cx.spawn(async move |view, cx| {
            if let Ok(record) = request.await {
                let _ = view.update(cx, |view, cx| {
                    view.record = Some(record);
                    cx.emit(RushSaved);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let view = cx.entity().downgrade();
        match self.rush {
            Some(rush) if self.over => {
                RushResult::new(rush.solved(), rush.kind(), self.record, move |_, cx| {
                    let _ = view.update(cx, |view, cx| view.start(rush.kind(), cx));
                })
                .into_any_element()
            }
            Some(rush) => {
                let status = self
                    .solver
                    .read(cx)
                    .status()
                    .unwrap_or_else(|| ("Next puzzle…".to_owned(), false));
                RushScore::new(
                    rush.solved(),
                    rush.strikes(),
                    rush.time_left(self.elapsed()),
                    status,
                )
                .problem(self.problem.clone(), move |_, cx| {
                    let _ = view.update(cx, |view, cx| view.retry(cx));
                })
                .into_any_element()
            }
            None => div().into_any_element(),
        }
    }
}

impl RushView {
    pub fn on_back(&mut self, on_back: impl Fn(&mut Window, &mut App) + 'static) {
        self.on_back = Rc::new(on_back);
    }
}

/// The count's number, large in a dark circle over the middle of the board.
fn countdown_overlay(count: u8) -> Div {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .size(px(160.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(black().opacity(0.65))
                .text_color(white())
                .text_size(px(96.))
                .font_weight(FontWeight::EXTRA_BOLD)
                .child(count.to_string()),
        )
}

impl Render for RushView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let on_back = Rc::clone(&self.on_back);
        let title = PanelTitle::new(RUSH_PICTURE, "Puzzle Rush")
            .on_back(move |window, cx| on_back(window, cx));
        let board = div()
            .relative()
            .size_full()
            .child(self.solver.clone())
            .children(self.countdown.map(countdown_overlay));
        board_page(
            BoardColumn::new(board),
            SidePanel::new().child(title).child(self.panel(cx)),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui_kit::{Entity, TestAppContext, VisualTestContext};

    use super::{COUNT_FROM, COUNT_STEP, MISS_PAUSE, NEXT_DELAY, RushView};
    use crate::{
        enums::puzzles::RushKind,
        testing::{click, play_and_wait, puzzle_server, wait_until},
    };

    fn has_puzzle(view: &Entity<RushView>, cx: &mut VisualTestContext) -> bool {
        view.update(cx, |view, cx| view.solver().read(cx).run().is_some())
    }

    fn started(
        cx: &mut TestAppContext,
        kind: RushKind,
    ) -> (Entity<RushView>, &mut VisualTestContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            RushView::new(puzzle_server(), Rc::new(|_| Some("abc".to_owned())), cx)
        });
        view.update(cx, |view, cx| view.start(kind, cx));
        count_through(cx);
        wait_until(cx, |cx| has_puzzle(&view, cx));
        (view, cx)
    }

    /// The 3, 2, 1 before a rush, a second at a time.
    fn count_through(cx: &mut VisualTestContext) {
        for _ in 0..COUNT_FROM {
            cx.executor().advance_clock(COUNT_STEP);
            cx.run_until_parked();
        }
    }

    #[gpui_kit::test]
    fn a_solved_puzzle_scores_and_the_next_one_follows(cx: &mut TestAppContext) {
        let (view, cx) = started(cx, RushKind::ThreeMinutes);
        play_and_wait(cx, "square-h5", "square-e8");
        play_and_wait(cx, "square-e1", "square-e8");
        click(cx, "square-e8");
        click(cx, "square-f8");
        cx.executor().advance_clock(NEXT_DELAY);
        wait_until(cx, |cx| has_puzzle(&view, cx));
        let rush = view.update(cx, |view, _| view.rush()).expect("a rush");
        assert_eq!(rush.solved(), 1);
        assert_eq!(rush.strikes(), 0);
        assert!(!view.update(cx, |view, _| view.is_over()));
    }

    #[gpui_kit::test]
    fn three_wrong_moves_end_a_survival_rush_and_save_its_score(cx: &mut TestAppContext) {
        let (view, cx) = started(cx, RushKind::Survival);
        for strike in 1..=3 {
            wait_until(cx, |cx| has_puzzle(&view, cx));
            click(cx, "square-h5");
            click(cx, "square-g6");
            let strikes = view.update(cx, |view, _| view.rush().map(|rush| rush.strikes()));
            assert_eq!(strikes, Some(strike));
            // The wrong move shows with its miss before the rush moves on.
            cx.executor().advance_clock(MISS_PAUSE);
            cx.run_until_parked();
        }
        assert!(view.update(cx, |view, _| view.is_over()));
        assert!(!has_puzzle(&view, cx), "the board empties at the end");
        wait_until(cx, |cx| view.update(cx, |view, _| view.record.is_some()));
        let record = view.update(cx, |view, _| view.record);
        assert_eq!(record.map(|record| record.new_best), Some(true));
    }

    /// A server whose narrow bands are all used up: only a wide band has a puzzle left.
    fn worn_out_server() -> crate::clients::server::ServerClient {
        use axum::{
            Json, Router,
            extract::Query,
            http::StatusCode,
            response::{IntoResponse, Response},
            routing::get,
        };
        use serde_json::json;

        #[derive(serde::Deserialize)]
        struct Band {
            min_rating: i32,
            max_rating: i32,
        }

        crate::clients::server::ServerClient::serving(Router::new().route(
            "/api/puzzles/next",
            get(|Query(band): Query<Band>| async move {
                if band.max_rating - band.min_rating <= 100 {
                    let problem = json!({
                        "type": "/problems/no-puzzle-in-range",
                        "title": "No new puzzle in that range: try a wider one",
                        "status": 404
                    });
                    return (StatusCode::NOT_FOUND, Json(problem)).into_response();
                }
                let puzzle = json!({
                    "id": "wide", "rating": 600, "plays": 1, "themes": [],
                    "fen": "7k/pp2r1p1/2pb3r/3p1B1Q/3N4/8/P2q2PP/4R2K w - - 1 1",
                    "last_move": "h7h8",
                    "solution": ["h5e8", "e7e8", "e1e8", "d6f8", "e8f8"]
                });
                let response: Response = Json(puzzle).into_response();
                response
            }),
        ))
    }

    #[gpui_kit::test]
    fn a_band_with_nothing_new_left_is_widened(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            RushView::new(worn_out_server(), Rc::new(|_| Some("abc".to_owned())), cx)
        });
        view.update(cx, |view, cx| view.start(RushKind::Survival, cx));
        count_through(cx);
        wait_until(cx, |cx| has_puzzle(&view, cx));
        let problem = view.update(cx, |view, _| view.problem.clone());
        assert_eq!(problem, None);
    }

    #[gpui_kit::test]
    fn without_a_server_the_rush_says_so_and_can_ask_again(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let http = crate::clients::http::HttpRuntime::new().expect("a runtime");
        let nowhere = crate::clients::server::ServerClient::new(http, "http://127.0.0.1:9");
        let (view, cx) = cx.add_window_view(|_, cx| {
            RushView::new(nowhere, Rc::new(|_| Some("abc".to_owned())), cx)
        });
        view.update(cx, |view, cx| view.start(RushKind::Survival, cx));
        wait_until(cx, |cx| view.update(cx, |view, _| view.problem.is_some()));
        view.update(cx, |view, cx| view.retry(cx));
        assert_eq!(view.update(cx, |view, _| view.problem.clone()), None);
        wait_until(cx, |cx| view.update(cx, |view, _| view.problem.is_some()));
        assert!(!has_puzzle(&view, cx));
    }

    #[gpui_kit::test]
    fn a_rush_counts_down_from_three_before_its_first_puzzle(cx: &mut TestAppContext) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            RushView::new(puzzle_server(), Rc::new(|_| Some("abc".to_owned())), cx)
        });
        view.update(cx, |view, cx| view.start(RushKind::ThreeMinutes, cx));
        let count = |cx: &mut VisualTestContext| view.update(cx, |view, _| view.countdown);
        assert_eq!(count(cx), Some(3));
        cx.executor().advance_clock(COUNT_STEP);
        cx.run_until_parked();
        assert_eq!(count(cx), Some(2));
        assert!(!has_puzzle(&view, cx), "no puzzle while counting");
        for _ in 0..2 {
            cx.executor().advance_clock(COUNT_STEP);
            cx.run_until_parked();
        }
        assert_eq!(count(cx), None);
        wait_until(cx, |cx| has_puzzle(&view, cx));
    }
}
