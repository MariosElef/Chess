use std::time::Duration;

use chess_domain::{
    Game, GameStatus, Move, Outcome, Position, Side, Square,
    engine::{Command, Eval, SearchInfo, UciMessage},
};
use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _,
        input::{InputEvent, TextareaState},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use crate::{
    clients::engine::{Engine, device_cores, locate_stockfish},
    components::{
        analysis::{
            EnginePanel, EvalBar, EvalGraph, FenInput, REVIEW_PICTURE, ReviewPanel, ShownVerdict,
        },
        board::{BoardColumn, board_page, copy_fen},
        history::{MoveList, NavigationBar},
        panel::{ChoiceTile, PanelHeader, SidePanel, StartButton},
        shell::ANALYSIS_PICTURE,
    },
    enums::{
        analysis::{EngineStatus, MoveClass},
        engine::EngineEvent,
        history::NavStep,
        sound::Sound,
    },
    models::{
        analysis::{GameReview, LineView, Pasted},
        board::{MoveMade, ShownPosition},
        engine::EngineSettings,
    },
    services::{
        analysis::{ReviewRun, class_count, line_view, read_pasted, review_engines},
        engine::SearchQueue,
        history::GameHistory,
    },
    views::{
        board::{BoardView, flip_button},
        history::{
            HistoryNavigation, Holding, hold_step, select_move, step_history, with_history_keys,
        },
        sound::{play_move_sound, play_sound},
        ticker::start_ticker,
    },
};

/// How many lines the engine panel shows, matching the engine's MultiPV.
const LINES: usize = 3;
/// How deep a review searches each position: enough for tactics, quick for a whole game.
const REVIEW_DEPTH: u32 = 16;
/// Each review engine's hash: its searches are short.
const REVIEW_HASH_MB: usize = 32;
/// The review asks for two lines: the second-best is what Great and Brilliant look at.
const REVIEW_LINES: usize = 2;
/// How often the board steps forward while the review plays through the game.
const REPLAY_STEP: Duration = Duration::from_millis(350);

/// One of a review's engines and the position it is searching.
struct ReviewWorker {
    engine: Engine,
    queue: SearchQueue,
    searching: Option<usize>,
    /// The lines of the search under way, as they come.
    lines: Vec<SearchInfo>,
    _task: Task<()>,
}

/// A review under way: a few engines searching the game's positions side by side.
struct Reviewer {
    workers: Vec<ReviewWorker>,
    run: ReviewRun,
    _replay: Task<()>,
}

/// Why the engine is not running: not installed, or stopped and could not restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EngineProblem {
    NotFound,
    Stopped,
}

/// A board to move freely on, with Stockfish analysing whatever position it shows.
pub struct AnalysisPage {
    history: GameHistory,
    /// The board, with what the player is doing on it and the queued premoves.
    board: Entity<BoardView>,
    _board_moves: Subscription,
    engine: Option<Engine>,
    queue: Option<SearchQueue>,
    /// Threads, memory and depth, from Settings.
    settings: EngineSettings,
    /// The switch: the engine runs while it is on and the page is on screen.
    enabled: bool,
    shown: bool,
    problem: Option<EngineProblem>,
    /// The engine stopped once and was started again; stopping again gives up on it.
    restarted: bool,
    /// The last review stopped before it finished: its engines died or would not start.
    review_problem: bool,
    /// The best lines of the current search, by MultiPV index.
    lines: [Option<SearchInfo>; LINES],
    /// The position the current lines belong to.
    analysed: Option<Position>,
    /// How the loaded game ended where it was played, when the moves alone do not say.
    ended: Option<Outcome>,
    /// The game reviewed, while it is the game on the board.
    review: Option<GameReview>,
    /// The review getting ready: the opening table being read before the engines start.
    review_start: Option<Task<()>>,
    reviewer: Option<Reviewer>,
    /// A key or navigation button held down.
    holding: Holding,
    fen_input: Entity<TextareaState>,
    fen_error: Option<String>,
    /// Hands engine output to the page; it ends when the engine's output does.
    _engine_task: Option<Task<()>>,
    _fen_events: Subscription,
    focus_handle: FocusHandle,
}

impl AnalysisPage {
    pub fn new(settings: EngineSettings, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let fen_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(1, 6)
                .placeholder("Paste a FEN or a PGN, then Load")
        });
        let _fen_events = cx.subscribe_in(&fen_input, window, on_fen_event);
        let page = cx.weak_entity();
        let board = cx.new(|board| {
            BoardView::new(
                move |cx| page.upgrade().map(|page| page.read(cx).shown_position()),
                board,
            )
        });
        let _board_moves = cx.subscribe(&board, |page, _, MoveMade(mv), cx| {
            page.play_move(*mv, cx);
        });
        Self {
            history: GameHistory::new(),
            board,
            _board_moves,
            engine: None,
            queue: None,
            settings,
            // Off until asked for: the engine works the computer hard.
            enabled: false,
            shown: false,
            problem: None,
            restarted: false,
            review_problem: false,
            lines: Default::default(),
            analysed: None,
            ended: None,
            review: None,
            review_start: None,
            reviewer: None,
            holding: Holding::default(),
            fen_input,
            fen_error: None,
            _engine_task: None,
            _fen_events,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    /// The shell says whether the page is on screen; a hidden page does not keep the engine busy.
    pub fn set_shown(&mut self, shown: bool, cx: &mut Context<Self>) {
        self.shown = shown;
        self.sync_engine(cx);
    }

    /// Opens a game from Play at the position it was showing, with how it ended.
    pub fn load_game(
        &mut self,
        game: &Game,
        (viewing, ended): (usize, Option<Outcome>),
        cx: &mut Context<Self>,
    ) {
        let reopened = game.truncated(game.moves().len());
        self.history = GameHistory::from_game(reopened, viewing);
        self.ended = ended;
        self.clear_review();
        self.after_navigation(cx);
    }

    /// New settings take effect by restarting the engine with them.
    pub fn set_engine_settings(&mut self, settings: EngineSettings, cx: &mut Context<Self>) {
        if settings == self.settings {
            return;
        }
        self.settings = settings;
        // Its listener goes with it, so the old engine's exit is not taken for a crash.
        self.engine = None;
        self.queue = None;
        self._engine_task = None;
        self.sync_engine(cx);
        cx.notify();
    }

    /// Back to the starting position with no moves.
    fn reset_board(&mut self, cx: &mut Context<Self>) {
        self.history = GameHistory::new();
        self.ended = None;
        self.clear_review();
        self.after_navigation(cx);
    }

    /// Takes the keyboard for the arrow keys, unless the click went to the FEN and PGN field.
    fn focus(&self, window: &mut Window, cx: &mut App) {
        if self.fen_input.focus_handle(cx).is_focused(window) {
            return;
        }
        self.focus_handle.focus(window, cx);
    }

    fn toggle_engine(&mut self, on: bool, cx: &mut Context<Self>) {
        self.enabled = on;
        if on {
            self.problem = None;
            self.restarted = false;
        }
        self.sync_engine(cx);
        cx.notify();
    }

    /// Searches the shown position while switched on, visible and not reviewing; else stops.
    fn sync_engine(&mut self, cx: &mut Context<Self>) {
        if self.enabled && self.shown && self.problem.is_none() && !self.is_reviewing() {
            self.ensure_engine(cx);
            self.request_shown();
        } else {
            self.stop_search();
        }
    }

    fn ensure_engine(&mut self, cx: &mut Context<Self>) {
        if self.engine.is_some() {
            return;
        }
        let Some(path) = locate_stockfish() else {
            self.problem = Some(EngineProblem::NotFound);
            return;
        };
        match Engine::spawn(&path) {
            Ok((engine, events)) => {
                let (queue, first) = SearchQueue::new(self.settings);
                self.engine = Some(engine);
                self.queue = Some(queue);
                self._engine_task = Some(listen(events, cx));
                self.send_all(&first);
            }
            Err(_) => self.problem = Some(EngineProblem::Stopped),
        }
    }

    /// Asks for the shown position; a finished one has nothing to search.
    fn request_shown(&mut self) {
        let shown = *self.history.shown();
        if shown.status() != GameStatus::Ongoing {
            self.stop_search();
            self.lines = Default::default();
            self.analysed = None;
            return;
        }
        if let Some(queue) = self.queue.as_mut() {
            let commands = queue.request(shown.to_string());
            self.send_all(&commands);
        }
    }

    fn stop_search(&mut self) {
        if let Some(queue) = self.queue.as_mut() {
            let commands = queue.stop();
            self.send_all(&commands);
        }
    }

    /// Sends commands in order; a new search clears the old lines and notes its position.
    fn send_all(&mut self, commands: &[Command]) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };
        if commands.iter().any(|command| engine.send(command).is_err()) {
            self.engine_failed();
            return;
        }
        if commands.iter().any(Command::is_go) {
            self.lines = Default::default();
            self.analysed = self
                .queue
                .as_ref()
                .and_then(SearchQueue::searching)
                .and_then(|fen| fen.parse().ok());
        }
    }

    /// The engine crashed: dropped and restarted when next wanted; a second failure gives up.
    fn engine_failed(&mut self) {
        self.engine = None;
        self.queue = None;
        self.lines = Default::default();
        self.analysed = None;
        if self.restarted {
            self.problem = Some(EngineProblem::Stopped);
            self.enabled = false;
        } else {
            self.restarted = true;
        }
    }

    fn on_engine_event(&mut self, event: EngineEvent, cx: &mut Context<Self>) {
        match event {
            EngineEvent::Message(message) => self.on_message(message),
            EngineEvent::Stopped => {
                // An engine already dropped, after a command could not be sent, ends here too:
                // that failure was counted then.
                if self.engine.is_some() {
                    self.engine_failed();
                }
                self.sync_engine(cx);
            }
        }
        cx.notify();
    }

    fn on_message(&mut self, message: UciMessage) {
        let Some(queue) = self.queue.as_mut() else {
            return;
        };
        let commands = match message {
            UciMessage::UciOk => queue.on_uciok(),
            UciMessage::ReadyOk => queue.on_readyok(),
            UciMessage::BestMove(_) => {
                // It searched: a later stop is a new failure, not the same one again.
                self.restarted = false;
                queue.on_bestmove()
            }
            UciMessage::Info(info) => {
                self.restarted = false;
                if queue.accepts_info() {
                    self.store_line(info);
                }
                Vec::new()
            }
            UciMessage::Other => Vec::new(),
        };
        self.send_all(&commands);
    }

    fn store_line(&mut self, info: SearchInfo) {
        if let Some(slot) = info
            .multipv
            .checked_sub(1)
            .and_then(|index| self.lines.get_mut(index))
        {
            *slot = Some(info);
        }
    }

    /// Any move or navigation: put the piece down and analyse the new position.
    fn after_position_change(&mut self, cx: &mut Context<Self>) {
        self.board.update(cx, |board, cx| board.put_down(cx));
        self.sync_engine(cx);
        cx.notify();
    }

    /// A position reached some other way than a move: the premoves no longer apply.
    fn after_navigation(&mut self, cx: &mut Context<Self>) {
        self.board.update(cx, |board, cx| board.reset(cx));
        self.after_position_change(cx);
    }

    /// How the game ended: by the rules, or as it ended where it was played.
    fn game_outcome(&self) -> Option<Outcome> {
        self.history.game().outcome().or(self.ended)
    }

    /// What the board shows: the position being looked at, with the engine's best move.
    fn shown_position(&self) -> ShownPosition {
        ShownPosition {
            position: *self.history.shown(),
            last_move: self.history.shown_last_move(),
            can_move: true,
            premoves: true,
            player: None,
            engine_arrow: self.engine_arrow().filter(|_| self.best_arrow().is_none()),
            best_arrow: self.best_arrow(),
            outcome: self
                .history
                .is_live()
                .then(|| self.game_outcome())
                .flatten(),
            badge: self.badge(),
        }
    }

    /// Either side may move from any shown position, replacing what followed it.
    fn play_move(&mut self, mv: Move, cx: &mut Context<Self>) {
        if self.play_with_sound(mv, cx) {
            // Another move makes it another game, which has not ended the same way.
            self.ended = None;
            self.clear_review();
            self.run_premoves(cx);
            self.after_position_change(cx);
        }
        cx.notify();
    }

    /// Plays queued premoves while each is legal; one that is not drops its side's queue.
    fn run_premoves(&mut self, cx: &mut Context<Self>) {
        loop {
            let shown = *self.history.shown();
            let Some(mv) = self.board.update(cx, |board, _| board.next_premove(&shown)) else {
                break;
            };
            if !self.play_with_sound(mv, cx) {
                break;
            }
        }
        if self.history.game().is_over() {
            self.board.update(cx, |board, _| board.drop_all_premoves());
        }
    }

    /// Plays `mv` from the shown position with its sound; whether the game took it.
    fn play_with_sound(&mut self, mv: Move, cx: &App) -> bool {
        let before = *self.history.shown();
        let played = self.history.play_from_shown(mv).is_ok();
        if played {
            let ends_game = self.history.game().is_over();
            play_move_sound(
                &before,
                self.history.shown(),
                mv,
                Sound::MoveSelf,
                ends_game,
                cx,
            );
        }
        played
    }

    fn flip(&mut self, cx: &mut Context<Self>) {
        self.board.update(cx, |board, cx| board.flip(cx));
        cx.notify();
    }

    /// Reads the field: a FEN starts a fresh analysis, a PGN opens the whole game.
    fn load_fen(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.fen_input.read(cx).value();
        if text.trim().is_empty() {
            self.fen_error = None;
            cx.notify();
            return;
        }
        match read_pasted(&text) {
            Ok(pasted) => {
                self.history = match pasted {
                    Pasted::Position(position) => GameHistory::from_position(position),
                    Pasted::Game(game) => GameHistory::from_game(game, 0),
                };
                self.ended = None;
                self.clear_review();
                self.fen_error = None;
                self.fen_input
                    .update(cx, |input, cx| input.set_value("", window, cx));
                self.after_navigation(cx);
            }
            Err(problem) => {
                self.fen_error = Some(problem);
                cx.notify();
            }
        }
    }

    /// The evaluation for the bar: the engine's best line, or the result of a finished position.
    fn eval(&self) -> Option<Eval> {
        let shown = self.history.shown();
        match shown.status() {
            GameStatus::Checkmate { winner } => Some(Eval::Mate { winner, moves: 0 }),
            GameStatus::Stalemate => Some(Eval::Centipawns(0)),
            GameStatus::Ongoing => {
                let (best, analysed) = self.best_for_shown()?;
                Some(Eval::from_uci(best.score, analysed.side_to_move()))
            }
        }
    }

    /// The best line, if it belongs to the position on the board.
    fn best_for_shown(&self) -> Option<(&SearchInfo, Position)> {
        let analysed = self.analysed?;
        let best = self.lines[0].as_ref()?;
        analysed
            .is_same_position(self.history.shown())
            .then_some((best, analysed))
    }

    /// The best move as an arrow, while the engine is on and the move is legal on the board.
    fn engine_arrow(&self) -> Option<(Square, Square)> {
        if !self.enabled {
            return None;
        }
        let (best, _) = self.best_for_shown()?;
        let mv = *best.pv.first()?;
        self.history
            .shown()
            .legal_moves()
            .contains(&mv)
            .then_some((mv.from, mv.to))
    }

    fn engine_status(&self) -> EngineStatus {
        match (self.problem, self.enabled) {
            (Some(EngineProblem::NotFound), _) => EngineStatus::NotFound,
            (Some(EngineProblem::Stopped), _) => EngineStatus::Stopped,
            (None, false) => EngineStatus::Off,
            (None, true) => self.running_status(),
        }
    }

    fn running_status(&self) -> EngineStatus {
        if self.is_reviewing() {
            return EngineStatus::PausedForReview;
        }
        if self.history.shown().status() != GameStatus::Ongoing {
            return EngineStatus::Finished;
        }
        match self.best_for_shown() {
            Some((best, _)) => EngineStatus::Running {
                depth: best.depth,
                nps: best.nps,
            },
            None => EngineStatus::Starting,
        }
    }

    /// The lines that belong to the shown position, written out with their evaluations.
    fn line_views(&self) -> Vec<LineView> {
        let Some(analysed) = self.analysed else {
            return Vec::new();
        };
        if !analysed.is_same_position(self.history.shown()) {
            return Vec::new();
        }
        self.lines
            .iter()
            .flatten()
            .map(|info| line_view(info, &analysed))
            .collect()
    }

    /// Stockfish is switched on and was found: the switch reads on and the eval bar shows.
    fn engine_on(&self) -> bool {
        self.enabled && self.problem.is_none()
    }

    /// The board, with the eval bar on its left while Stockfish is on.
    fn board_column(&self, cx: &mut Context<Self>) -> BoardColumn {
        let flipped = self.board.read(cx).flipped();
        BoardColumn::new(self.board.clone())
            .when(self.engine_on(), |column| {
                column.left_strip(EvalBar::new(self.eval(), flipped))
            })
            .tool(flip_button("analysis-flip-board", cx, AnalysisPage::flip))
            .tool(copy_fen(self.history.shown()))
    }

    /// The review, the engine, the moves and the field to load a game.
    fn side_column(&self, page: WeakEntity<Self>, cx: &mut Context<Self>) -> SidePanel {
        SidePanel::new()
            .header(PanelHeader::new(ANALYSIS_PICTURE, "Analysis"))
            .child(self.header(cx))
            .child(EnginePanel::new(
                self.engine_on(),
                self.engine_status(),
                self.line_views(),
                toggle_engine(page.clone()),
            ))
            .child(self.middle(page.clone(), cx))
            .child(
                NavigationBar::new(
                    self.history.can_go_back(),
                    self.history.can_go_forward(),
                    step_history(page.clone()),
                )
                .on_hold(hold_step(page.clone())),
            )
            .child(FenInput::new(
                self.fen_input.clone(),
                self.fen_error.clone(),
                load_fen(page),
            ))
    }

    /// Between the fixed top and bottom: the moves, and once reviewed, the review over them.
    fn middle(&self, page: WeakEntity<Self>, cx: &mut Context<Self>) -> AnyElement {
        let moves = MoveList::new(
            self.history.sans().to_vec(),
            self.history.viewing(),
            select_move(page),
        )
        .classes(self.review_classes());
        match self.review_panel(cx) {
            None => moves.into_any_element(),
            Some(review) => div()
                .id("analysis-middle")
                .w_full()
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(
                    v_flex()
                        .w_full()
                        .gap_3()
                        .child(review)
                        .child(moves.unbounded()),
                )
                .into_any_element(),
        }
    }

    /// Starts a review of the game on the board; live analysis pauses until it is done.
    pub fn start_review(&mut self, cx: &mut Context<Self>) {
        if self.history.sans().is_empty() || self.is_reviewing() {
            return;
        }
        self.review_problem = false;
        if locate_stockfish().is_none() {
            self.problem = Some(EngineProblem::NotFound);
            cx.notify();
            return;
        }
        let game = self.history.game().clone();
        self.review = None;
        self.stop_search();
        self.review_start = Some(cx.spawn(async move |page, cx| {
            let run = cx
                .background_executor()
                .spawn(async move { ReviewRun::new(&game) })
                .await;
            let _ = page.update(cx, |page, cx| page.begin_review(run, cx));
        }));
        cx.notify();
    }

    fn is_reviewing(&self) -> bool {
        self.review_start.is_some() || self.reviewer.is_some()
    }

    /// Starts the review's engines, as many as the computer can spare, and the replay.
    fn begin_review(&mut self, run: ReviewRun, cx: &mut Context<Self>) {
        self.review_start = None;
        if let Some(review) = run.finish() {
            // Nothing to search: every position was already known, as a finished one is.
            self.complete_review(review, cx);
            return;
        }
        let Some(path) = locate_stockfish() else {
            self.problem = Some(EngineProblem::NotFound);
            cx.notify();
            return;
        };
        let mut workers = Vec::new();
        for _ in 0..review_engines(device_cores()) {
            let Ok((engine, events)) = Engine::spawn(&path) else {
                break;
            };
            // Its place among the workers that started: its messages are routed by it.
            let index = workers.len();
            let settings = EngineSettings {
                threads: 1,
                hash_mb: REVIEW_HASH_MB,
                depth: Some(REVIEW_DEPTH),
            };
            let (queue, first) = SearchQueue::new(settings);
            let mut worker = ReviewWorker {
                engine,
                queue: queue.with_options(vec![("MultiPV", REVIEW_LINES.to_string())]),
                searching: None,
                lines: Vec::new(),
                _task: listen_review(index, events, cx),
            };
            if first
                .iter()
                .all(|command| worker.engine.send(command).is_ok())
            {
                workers.push(worker);
            }
        }
        if workers.is_empty() {
            self.review_problem = true;
            // Live analysis waited for the review: with none to run, it comes back.
            self.sync_engine(cx);
            cx.notify();
            return;
        }
        self.reviewer = Some(Reviewer {
            workers,
            run,
            _replay: start_ticker(REPLAY_STEP, cx, AnalysisPage::replay_step),
        });
        self.show_position(0, cx);
    }

    fn on_review_event(&mut self, worker: usize, event: EngineEvent, cx: &mut Context<Self>) {
        match event {
            EngineEvent::Message(message) => self.on_review_message(worker, message, cx),
            // With no review running, finished or never begun, an engine stopping is no failure.
            EngineEvent::Stopped if self.reviewer.is_some() => self.review_failed(cx),
            EngineEvent::Stopped => {}
        }
        cx.notify();
    }

    fn on_review_message(&mut self, index: usize, message: UciMessage, cx: &mut Context<Self>) {
        let Some(reviewer) = self.reviewer.as_mut() else {
            return;
        };
        let Some(worker) = reviewer.workers.get_mut(index) else {
            return;
        };
        match message {
            UciMessage::UciOk => {
                let commands = worker.queue.on_uciok();
                self.send_review(index, &commands, cx);
            }
            UciMessage::ReadyOk => {
                let commands = worker.queue.on_readyok();
                self.send_review(index, &commands, cx);
                self.give_work(index, cx);
            }
            UciMessage::Info(info) => {
                if worker.queue.accepts_info() {
                    worker.lines.retain(|line| line.multipv != info.multipv);
                    worker.lines.push(info);
                }
            }
            UciMessage::BestMove(_) => {
                let commands = worker.queue.on_bestmove();
                let lines = std::mem::take(&mut worker.lines);
                if let Some(position) = worker.searching.take() {
                    reviewer.run.store(position, &lines);
                }
                self.send_review(index, &commands, cx);
                self.give_work(index, cx);
            }
            UciMessage::Other => {}
        }
    }

    /// Hands the engine the next position to search; once all have values, the review is ready.
    fn give_work(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(reviewer) = self.reviewer.as_mut() else {
            return;
        };
        if let Some(review) = reviewer.run.finish() {
            self.complete_review(review, cx);
            return;
        }
        let Some((position, fen)) = reviewer.run.next_position() else {
            return;
        };
        let Some(worker) = reviewer.workers.get_mut(index) else {
            return;
        };
        worker.searching = Some(position);
        let commands = worker.queue.request(fen);
        self.send_review(index, &commands, cx);
    }

    /// The review is ready: the engines go, live analysis comes back, and the board starts over.
    fn complete_review(&mut self, review: GameReview, cx: &mut Context<Self>) {
        self.review = Some(review);
        self.review_problem = false;
        self.reviewer = None;
        play_sound(Sound::Notify, cx);
        self.show_position(0, cx);
    }

    /// Steps the board a move at a time as far as the review has got; whether to keep stepping.
    fn replay_step(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(reviewer) = &self.reviewer else {
            return false;
        };
        let known = reviewer.run.known_from_start().saturating_sub(1);
        if self.history.viewing() < known {
            self.navigate(NavStep::Forward, cx);
        }
        true
    }

    /// Shows the game's position at `index`, 0 being the start, with its move's sound.
    fn show_position(&mut self, index: usize, cx: &mut Context<Self>) {
        self.walk(
            |history| match index.checked_sub(1) {
                Some(move_index) => history.jump_to_move(move_index),
                None => history.step(NavStep::Start),
            },
            true,
            cx,
        );
    }

    fn send_review(&mut self, index: usize, commands: &[Command], cx: &mut Context<Self>) {
        let Some(worker) = self
            .reviewer
            .as_mut()
            .and_then(|reviewer| reviewer.workers.get_mut(index))
        else {
            return;
        };
        if commands
            .iter()
            .any(|command| worker.engine.send(command).is_err())
        {
            self.review_failed(cx);
        }
    }

    /// A review engine died: the review is dropped, its button says so, and live analysis returns.
    fn review_failed(&mut self, cx: &mut Context<Self>) {
        self.reviewer = None;
        self.review_problem = true;
        self.sync_engine(cx);
    }

    /// A review belongs to one game: any change to the game, or a new one, drops it.
    fn clear_review(&mut self) {
        self.review_problem = false;
        self.review = None;
        self.review_start = None;
        self.reviewer = None;
    }

    /// Each move's class, once reviewed.
    fn review_classes(&self) -> Vec<MoveClass> {
        self.review.as_ref().map_or_else(Vec::new, |review| {
            review.moves.iter().map(|reviewed| reviewed.class).collect()
        })
    }

    /// The engine's choice where the move played was not it, as chess.com shows it.
    fn best_arrow(&self) -> Option<(Square, Square)> {
        let (_, better) = self.better_move()?;
        Some((better.from, better.to))
    }

    /// What the engine preferred to the move that led to the shown position, with its index.
    fn better_move(&self) -> Option<(usize, Move)> {
        let index = self.history.viewing().checked_sub(1)?;
        let reviewed = self.review.as_ref()?.moves.get(index)?;
        let played = *self.history.game().moves().get(index)?;
        let best = reviewed.best.filter(|&best| best != played)?;
        let shows_better = !matches!(reviewed.class, MoveClass::Book | MoveClass::Forced);
        shows_better.then_some((index, best))
    }

    /// The class of the move that led to the shown position, on the square it went to.
    fn badge(&self) -> Option<(Square, MoveClass)> {
        let index = self.history.viewing().checked_sub(1)?;
        let class = self.review.as_ref()?.moves.get(index)?.class;
        let mv = *self.history.game().moves().get(index)?;
        Some((mv.to, class))
    }

    fn review_panel(&self, cx: &mut Context<Self>) -> Option<ReviewPanel> {
        let review = self.review.as_ref()?;
        let counts = MoveClass::ALL
            .into_iter()
            .map(|class| {
                (
                    class,
                    class_count(review, Side::White, class),
                    class_count(review, Side::Black, class),
                )
            })
            .collect();
        let page = cx.entity().downgrade();
        let graph = EvalGraph::new(
            review.shares.clone(),
            self.history.viewing(),
            move |position, _, cx| {
                let _ = page.update(cx, |page, cx| page.show_position(position, cx));
            },
        );
        Some(ReviewPanel::new(
            (review.accuracy, review.ratings),
            counts,
            (self.shown_verdict(), graph),
        ))
    }

    /// What the move on the board was, and what was best when it was not.
    fn shown_verdict(&self) -> Option<ShownVerdict> {
        let index = self.history.viewing().checked_sub(1)?;
        let reviewed = self.review.as_ref()?.moves.get(index)?;
        let game = self.history.game();
        let best = self
            .better_move()
            .and_then(|(_, best)| game.positions().get(index)?.san(best).ok());
        Some(ShownVerdict {
            class: reviewed.class,
            san: game.sans().get(index)?.clone(),
            best,
        })
    }

    /// Game Review, large and green with its star; it counts the positions while it runs.
    fn review_button(&self, cx: &mut Context<Self>) -> StartButton {
        let label = match (&self.reviewer, self.is_reviewing()) {
            (Some(reviewer), _) => {
                let (done, total) = reviewer.run.progress();
                format!("Reviewing {done}/{total}")
            }
            (None, true) => "Reviewing…".to_owned(),
            (None, false) => "Game Review".to_owned(),
        };
        let page = cx.entity().downgrade();
        StartButton::new("review-game", label, move |_, cx| {
            let _ = page.update(cx, |page, cx| page.start_review(cx));
        })
        .picture(REVIEW_PICTURE)
        .busy(self.is_reviewing())
    }

    /// Top of the panel: Game Review until reviewed, why a review stopped, and Reset board.
    fn header(&self, cx: &mut Context<Self>) -> Div {
        let reset = cx.entity().downgrade();
        let reviewable = !self.history.sans().is_empty() && self.review.is_none();
        v_flex()
            .gap_2()
            .when(reviewable, |this| this.child(self.review_button(cx)))
            .when(self.review_problem, |this| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child("The review stopped: try again"),
                )
            })
            .child(
                ChoiceTile::new("reset-board", "Reset board", false, move |_, cx| {
                    let _ = reset.update(cx, |page, cx| page.reset_board(cx));
                })
                .icon(IconName::RefreshCw)
                .height(px(40.)),
            )
    }
}

impl HistoryNavigation for AnalysisPage {
    fn history_mut(&mut self) -> Option<&mut GameHistory> {
        Some(&mut self.history)
    }

    fn holding(&mut self) -> &mut Holding {
        &mut self.holding
    }

    fn after_history_move(&mut self, cx: &mut Context<Self>) {
        self.after_navigation(cx);
    }
}

impl Render for AnalysisPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = cx.entity().downgrade();
        with_history_keys(v_flex(), cx)
            .track_focus(&self.focus_handle)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|page, _, window, cx| page.focus(window, cx)),
            )
            .size_full()
            .child(board_page(
                self.board_column(cx),
                self.side_column(page, cx),
            ))
    }
}

/// Reads one review engine's output until it ends or the page is gone.
fn listen_review(
    worker: usize,
    events: gpui_kit::base::async_util::Receiver<EngineEvent>,
    cx: &mut Context<AnalysisPage>,
) -> Task<()> {
    cx.spawn(async move |page, cx| {
        while let Ok(event) = events.recv().await {
            if page
                .update(cx, |page, cx| page.on_review_event(worker, event, cx))
                .is_err()
            {
                break;
            }
        }
    })
}

/// Enter in the FEN field loads it, like the Load button.
fn on_fen_event(
    page: &mut AnalysisPage,
    _: &Entity<TextareaState>,
    event: &InputEvent,
    window: &mut Window,
    cx: &mut Context<AnalysisPage>,
) {
    if let InputEvent::PressEnter { .. } = event {
        page.load_fen(window, cx);
    }
}

/// Reads engine output until it ends or the page is gone.
fn listen(
    events: gpui_kit::base::async_util::Receiver<EngineEvent>,
    cx: &mut Context<AnalysisPage>,
) -> Task<()> {
    cx.spawn(async move |page, cx| {
        while let Ok(event) = events.recv().await {
            if page
                .update(cx, |page, cx| page.on_engine_event(event, cx))
                .is_err()
            {
                break;
            }
        }
    })
}

// The page outlives what it renders; a failed update means the window is closing.

fn toggle_engine(page: WeakEntity<AnalysisPage>) -> impl Fn(bool, &mut Window, &mut App) + 'static {
    move |on, _, cx| {
        let _ = page.update(cx, |page, cx| page.toggle_engine(on, cx));
    }
}

fn load_fen(page: WeakEntity<AnalysisPage>) -> impl Fn(&mut Window, &mut App) + 'static {
    move |window, cx| {
        let _ = page.update(cx, |page, cx| page.load_fen(window, cx));
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gpui_kit::{
        Entity, Focusable as _, Modifiers, MouseButton, TestAppContext, VisualTestContext,
    };

    use chess_domain::{Game, Move};

    use super::AnalysisPage;
    use crate::{
        clients::engine::locate_stockfish,
        enums::{analysis::MoveClass, engine::EngineEvent, history::NavStep, sound::Sound},
        models::engine::EngineSettings,
        testing::{centre_of, click, drag},
        views::sound::{played_sounds, record_sounds},
    };
    use crate::{components::board::SLIDE_TIME, views::history::HistoryNavigation as _};

    fn opened(cx: &mut TestAppContext) -> (Entity<AnalysisPage>, &mut VisualTestContext) {
        cx.update(gpui_kit::init);
        cx.add_window_view(|window, cx| AnalysisPage::new(EngineSettings::for_cores(2), window, cx))
    }

    #[gpui_kit::test]
    fn the_promotion_choices_hang_from_the_promotion_square(cx: &mut TestAppContext) {
        let (page, cx) = opened(cx);
        page.update(cx, |page, cx| {
            let position = "k7/4P3/8/8/8/8/8/4K3 w - - 0 1".parse().expect("valid");
            page.history = crate::services::history::GameHistory::from_position(position);
            cx.notify();
        });
        cx.run_until_parked();
        drag(cx, "square-e7", "square-e8");
        let square = cx.debug_bounds("square-e8").expect("e8");
        let queen = cx
            .debug_bounds("promote-0")
            .expect("the queen, offered first");
        let bishop = cx
            .debug_bounds("promote-2")
            .expect("the bishop, offered last");
        let near = |a: gpui_kit::Pixels, b: gpui_kit::Pixels| (a - b).abs() < gpui_kit::px(2.);
        assert!(near(queen.origin.x, square.origin.x), "on the e-file");
        assert!(
            near(queen.origin.y, square.origin.y),
            "the queen on e8 itself"
        );
        assert!(near(queen.size.width, square.size.width), "one square wide");
        assert!(
            near(bishop.origin.y, square.origin.y + square.size.height * 3.),
            "the column runs down from e8: queen, knight, rook, bishop"
        );
        click(cx, "promote-3");
        page.update(cx, |page, _| {
            assert_eq!(page.history.sans().last().map(String::as_str), Some("e8=N"));
        });
    }

    #[gpui_kit::test]
    fn a_premove_that_mates_drops_every_other_premove(cx: &mut TestAppContext) {
        let (page, cx) = opened(cx);
        drag(cx, "square-f2", "square-f3");
        drag(cx, "square-e7", "square-e5");
        drag(cx, "square-d8", "square-h4");
        drag(cx, "square-a7", "square-a6");
        drag(cx, "square-g2", "square-g4");
        page.update(cx, |page, cx| {
            assert_eq!(page.history.sans(), ["f3", "e5", "g4", "Qh4#"]);
            assert!(
                page.board.read(cx).premoves().is_empty(),
                "a6 is dropped once the game is over"
            );
        });
    }

    #[gpui_kit::test]
    fn moves_and_refused_drops_make_their_sounds(cx: &mut TestAppContext) {
        let (_, cx) = opened(cx);
        cx.update(|_, cx| record_sounds(cx));
        drag(cx, "square-e2", "square-e5");
        drag(cx, "square-e2", "square-e4");
        drag(cx, "square-d7", "square-d5");
        drag(cx, "square-e4", "square-d5");
        let played = cx.update(|_, cx| played_sounds(cx));
        assert_eq!(
            played,
            [
                Sound::Illegal,
                Sound::MoveSelf,
                Sound::MoveSelf,
                Sound::Capture
            ]
        );
    }

    #[gpui_kit::test]
    fn a_mate_sounds_its_check_then_the_end(cx: &mut TestAppContext) {
        let (_, cx) = opened(cx);
        for (from, to) in [
            ("square-f2", "square-f3"),
            ("square-e7", "square-e5"),
            ("square-g2", "square-g4"),
        ] {
            drag(cx, from, to);
        }
        cx.update(|_, cx| record_sounds(cx));
        drag(cx, "square-d8", "square-h4");
        let played = cx.update(|_, cx| played_sounds(cx));
        assert_eq!(played, [Sound::Check, Sound::GameEnd]);
    }

    #[gpui_kit::test]
    fn stepping_through_the_game_plays_each_moves_sound(cx: &mut TestAppContext) {
        let (page, cx) = opened(cx);
        for (from, to) in [
            ("square-e2", "square-e4"),
            ("square-d7", "square-d5"),
            ("square-e4", "square-d5"),
        ] {
            drag(cx, from, to);
        }
        cx.update(|_, cx| record_sounds(cx));
        page.update(cx, |page, cx| {
            page.navigate(NavStep::Start, cx);
            page.navigate(NavStep::Forward, cx);
            page.navigate(NavStep::Live, cx);
            page.navigate(NavStep::Live, cx);
            page.select_move(1, cx);
        });
        let played = cx.update(|_, cx| played_sounds(cx));
        assert_eq!(
            played,
            [Sound::MoveSelf, Sound::Capture, Sound::MoveSelf],
            "the start is silent, e4 moves, exd5 captures, staying put is silent, d5 moves"
        );
    }

    #[gpui_kit::test]
    fn stepping_slides_the_piece_but_a_move_made_on_the_board_does_not(cx: &mut TestAppContext) {
        let (page, cx) = opened(cx);
        drag(cx, "square-e2", "square-e4");
        cx.run_until_parked();
        assert!(
            cx.debug_bounds("piece-e4").is_some(),
            "the dropped pawn stands at once"
        );
        page.update(cx, |page, cx| page.navigate(NavStep::Back, cx));
        cx.run_until_parked();
        assert!(
            cx.debug_bounds("piece-e2").is_none(),
            "the pawn slides back to e2 before it stands there"
        );
        cx.executor().advance_clock(SLIDE_TIME * 2);
        cx.run_until_parked();
        assert!(cx.debug_bounds("piece-e2").is_some(), "and then it is home");
    }

    #[gpui_kit::test]
    fn holding_forward_keeps_stepping_until_let_go(cx: &mut TestAppContext) {
        let (page, cx) = opened(cx);
        page.update(cx, |page, cx| {
            page.load_game(&fools_mate(), (0, None), cx);
        });
        let forward = centre_of(cx, "held-nav-forward");
        cx.simulate_mouse_down(forward, MouseButton::Left, Modifiers::none());
        cx.executor()
            .advance_clock(Duration::from_millis(400 + 60 * 2 + 10));
        cx.run_until_parked();
        let held = page.read_with(cx, |page, _| page.history.viewing());
        assert!(held >= 3, "the click and the repeats, now at {held}");
        cx.simulate_mouse_up(forward, MouseButton::Left, Modifiers::none());
        page.update(cx, |page, cx| page.navigate(NavStep::Start, cx));
        cx.executor().advance_clock(Duration::from_secs(1));
        cx.run_until_parked();
        assert_eq!(
            page.read_with(cx, |page, _| page.history.viewing()),
            0,
            "let go, it stops"
        );
    }

    #[gpui_kit::test]
    fn a_pasted_pgn_opens_the_whole_game_at_its_start(cx: &mut TestAppContext) {
        let (page, cx) = opened(cx);
        let pgn = "[Event \"Live Chess\"]\n[Result \"0-1\"]\n\n1. f3 e5 2. g4 Qh4# 0-1";
        let field = page.read_with(cx, |page, _| page.fen_input.clone());
        field.update_in(cx, |field, window, cx| field.set_value(pgn, window, cx));
        page.update_in(cx, |page, window, cx| page.load_fen(window, cx));
        page.read_with(cx, |page, _| {
            assert_eq!(page.history.sans(), ["f3", "e5", "g4", "Qh4#"]);
            assert_eq!(
                page.history.viewing(),
                0,
                "from the start, ready to step or review"
            );
            assert!(page.fen_error.is_none());
        });
        field.update_in(cx, |field, window, cx| {
            field.set_value("1. e4 e5 2. Ke3", window, cx)
        });
        page.update_in(cx, |page, window, cx| page.load_fen(window, cx));
        page.read_with(cx, |page, _| {
            assert_eq!(
                page.fen_error.as_deref(),
                Some("Not a valid PGN: 2. Ke3 is not a legal move there")
            );
        });
    }

    #[gpui_kit::test]
    fn clicking_the_fen_field_keeps_it_for_typing(cx: &mut TestAppContext) {
        let (page, cx) = opened(cx);
        let field = page.read_with(cx, |page, _| page.fen_input.clone());
        let bounds = cx.debug_bounds("fen-field").expect("the field");
        cx.simulate_click(bounds.center(), Modifiers::none());
        cx.run_until_parked();
        let focused = cx.update(|window, cx| field.focus_handle(cx).is_focused(window));
        assert!(
            focused,
            "the page must not take the keyboard back from the field"
        );
        cx.simulate_input("1. e4 e5");
        cx.run_until_parked();
        let typed = field.read_with(cx, |field, _| field.value().to_string());
        assert_eq!(typed, "1. e4 e5");
    }

    #[gpui_kit::test]
    fn clicks_premove_on_the_analysis_board(cx: &mut TestAppContext) {
        let (page, cx) = opened(cx);
        for selector in ["square-e7", "square-e5", "square-e2", "square-e4"] {
            click(cx, selector);
        }
        page.update(cx, |page, _| assert_eq!(page.history.sans(), ["e4", "e5"]));
    }

    #[gpui_kit::test]
    fn drags_premove_on_the_analysis_board(cx: &mut TestAppContext) {
        let (page, cx) = opened(cx);
        drag(cx, "square-g8", "square-f6");
        drag(cx, "square-f6", "square-g4");
        drag(cx, "square-d2", "square-d4");
        page.update(cx, |page, cx| {
            assert_eq!(page.history.sans(), ["d4", "Nf6"]);
            assert!(
                !page.board.read(cx).premoves().is_empty(),
                "Ng4 waits for White's next move"
            );
        });
        drag(cx, "square-c2", "square-c4");
        page.update(cx, |page, _| {
            assert_eq!(page.history.sans(), ["d4", "Nf6", "c4", "Ng4"]);
        });
    }

    /// Fool's mate: f3 e5 g4 Qh4#.
    fn fools_mate() -> Game {
        let mut game = Game::new();
        for uci in ["f2f3", "e7e5", "g2g4", "d8h4"] {
            let mv: Move = uci.parse().expect("a move");
            game.play(mv).expect("legal");
        }
        game
    }

    #[gpui_kit::test]
    fn a_review_with_the_real_engine_finds_the_blunder(cx: &mut TestAppContext) {
        if locate_stockfish().is_none() {
            return;
        }
        cx.executor().allow_parking();
        let (page, cx) = opened(cx);
        cx.update(|_, cx| record_sounds(cx));
        page.update(cx, |page, cx| {
            page.load_game(&fools_mate(), (4, None), cx);
            page.start_review(cx);
        });
        for _ in 0..3000 {
            cx.run_until_parked();
            if page.read_with(cx, |page, _| page.review.is_some()) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        page.read_with(cx, |page, _| {
            let review = page.review.as_ref().expect("the review finished");
            assert_eq!(review.moves[2].class, MoveClass::Blunder, "g4 allows mate");
            assert!(
                review
                    .accuracy
                    .iter()
                    .all(|side| (0.0..=100.0).contains(side))
            );
            assert!(
                review.accuracy[1] > review.accuracy[0],
                "Black played better"
            );

            assert!(page.reviewer.is_none(), "the review's engine is gone");
            assert_eq!(
                page.history.viewing(),
                0,
                "the review starts from the beginning"
            );
        });
        let played = cx.update(|_, cx| played_sounds(cx));
        assert_eq!(
            played.last(),
            Some(&Sound::Notify),
            "then the review is ready"
        );
        page.update(cx, |page, cx| {
            page.select_move(3, cx);
            let (square, _) = page.badge().expect("Qh4# has its icon");
            assert_eq!(square.to_string(), "h4");
            page.select_move(2, cx);
            let shown = page.shown_position();
            let (from, to) = shown
                .best_arrow
                .expect("g4 was not best: the better move shows");
            assert_ne!(
                (from.to_string(), to.to_string()),
                ("g2".to_owned(), "g4".to_owned())
            );
            assert!(shown.engine_arrow.is_none(), "one arrow at a time");
        });
    }

    #[gpui_kit::test]
    fn a_resigned_game_keeps_its_ending_until_the_board_changes(cx: &mut TestAppContext) {
        let (page, cx) = opened(cx);
        let mut game = Game::new();
        for (from, to) in [("e2", "e4"), ("e7", "e5")] {
            let mv = Move::new(
                from.parse().expect("a square"),
                to.parse().expect("a square"),
            );
            game.play(mv).expect("legal");
        }
        let resigned = Some(chess_domain::Outcome::Resignation {
            winner: chess_domain::Side::White,
        });
        page.update(cx, |page, cx| page.load_game(&game, (2, resigned), cx));
        let shown = page.read_with(cx, |page, _| page.shown_position().outcome);
        assert_eq!(shown, resigned, "the kings wear it on the final position");
        page.update(cx, |page, cx| page.reset_board(cx));
        let shown = page.read_with(cx, |page, _| page.shown_position().outcome);
        assert_eq!(shown, None, "a new board has not ended");
    }

    #[gpui_kit::test]
    fn a_failed_review_message_goes_once_the_board_changes(cx: &mut TestAppContext) {
        let (page, cx) = opened(cx);
        page.update(cx, |page, cx| {
            page.review_problem = true;
            page.reset_board(cx);
        });
        assert!(!page.read_with(cx, |page, _| page.review_problem));
    }

    #[gpui_kit::test]
    fn an_engine_stopping_with_no_review_running_is_no_failure(cx: &mut TestAppContext) {
        let (page, cx) = opened(cx);
        page.update(cx, |page, cx| {
            page.on_review_event(0, EngineEvent::Stopped, cx);
        });
        assert!(
            !page.read_with(cx, |page, _| page.review_problem),
            "no review was running, so nothing stopped it"
        );
    }
}
