use chess_domain::{
    Game, GameStatus, Move, Position, Side, Square,
    engine::{Command, Eval, SearchInfo, UciMessage},
};
use gpui_kit::{
    assets::IconName,
    component::{
        Disableable as _,
        button::{Button, ButtonVariants},
        h_flex,
        input::{InputEvent, InputState},
        label::Label,
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use crate::{
    clients::engine::{Engine, locate_stockfish},
    components::{
        analysis::{EnginePanel, EvalBar, FenInput, ReviewPanel, ShownVerdict},
        history::{MoveList, NavigationBar, PositionPanel},
    },
    enums::{
        analysis::{EngineStatus, MoveClass},
        engine::EngineEvent,
        history::NavStep,
        sound::Sound,
    },
    models::{
        analysis::{GameReview, LineView},
        board::{MoveMade, ShownPosition},
        engine::EngineSettings,
    },
    services::{
        analysis::{ReviewRun, class_count, line_view, warm_opening_book},
        engine::SearchQueue,
        history::GameHistory,
    },
    views::{
        board::BoardView,
        history::{
            HistoryNavigation, Holding, hold_step, select_move, step_history, with_history_keys,
        },
        sound::{play_move_sound, play_sound},
    },
};

/// How many lines the engine panel shows, matching the engine's MultiPV.
const LINES: usize = 3;
/// How deep a review searches each position: deep enough to see tactics, quick enough for a
/// whole game in a few seconds.
const REVIEW_DEPTH: u32 = 16;
/// The review's own hash, smaller than the live engine's: each search is short.
const REVIEW_HASH_MB: usize = 128;

/// A review under way: its own engine, searching the game's positions one after another.
struct Reviewer {
    engine: Engine,
    queue: SearchQueue,
    run: ReviewRun,
    /// The lines of the position being searched, as they come.
    lines: Vec<SearchInfo>,
    _task: Task<()>,
}

/// Why the engine is not running although it is switched on.
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
    /// The best lines of the current search, by MultiPV index.
    lines: [Option<SearchInfo>; LINES],
    /// The position the current lines belong to.
    analysed: Option<Position>,
    /// The game reviewed, while it is the game on the board.
    review: Option<GameReview>,
    reviewer: Option<Reviewer>,
    /// A key or navigation button held down.
    holding: Holding,
    fen_input: Entity<InputState>,
    fen_error: Option<String>,
    /// Hands engine output to the page; it ends when the engine's output does.
    _engine_task: Option<Task<()>>,
    _fen_events: Subscription,
    focus_handle: FocusHandle,
}

impl AnalysisPage {
    pub fn new(settings: EngineSettings, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let fen_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Paste a FEN and press Enter"));
        let _fen_events = cx.subscribe_in(&fen_input, window, on_fen_event);
        let page = cx.weak_entity();
        let board = cx.new(|_| {
            BoardView::new(move |cx| page.upgrade().map(|page| page.read(cx).shown_position()))
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
            enabled: true,
            shown: false,
            problem: None,
            lines: Default::default(),
            analysed: None,
            review: None,
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

    /// Opens a game from Play, at the position it was showing.
    pub fn load_game(&mut self, game: &Game, viewing: usize, cx: &mut Context<Self>) {
        let reopened = game.truncated(game.moves().len());
        self.history = GameHistory::from_game(reopened, viewing);
        self.clear_review();
        self.after_navigation(cx);
    }

    /// New settings take effect by restarting the engine with them.
    pub fn set_engine_settings(&mut self, settings: EngineSettings, cx: &mut Context<Self>) {
        if settings == self.settings {
            return;
        }
        self.settings = settings;
        self.engine = None;
        self.queue = None;
        self.sync_engine(cx);
        cx.notify();
    }

    /// Back to the starting position with no moves.
    fn reset_board(&mut self, cx: &mut Context<Self>) {
        self.history = GameHistory::new();
        self.clear_review();
        self.after_navigation(cx);
    }

    fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus_handle.focus(window, cx);
    }

    fn toggle_engine(&mut self, on: bool, cx: &mut Context<Self>) {
        self.enabled = on;
        if on {
            self.problem = None;
        }
        self.sync_engine(cx);
        cx.notify();
    }

    /// Searches the shown position while switched on and visible, and no review is running;
    /// otherwise stops searching.
    fn sync_engine(&mut self, cx: &mut Context<Self>) {
        if self.enabled && self.shown && self.problem.is_none() && self.reviewer.is_none() {
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

    /// The engine crashed or could not be written to: drop it and say so.
    fn engine_failed(&mut self) {
        self.engine = None;
        self.queue = None;
        self.problem = Some(EngineProblem::Stopped);
    }

    fn on_engine_event(&mut self, event: EngineEvent, cx: &mut Context<Self>) {
        match event {
            EngineEvent::Message(message) => self.on_message(message),
            EngineEvent::Stopped => self.engine_failed(),
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
            UciMessage::BestMove(_) => queue.on_bestmove(),
            UciMessage::Info(info) => {
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
            badge: self.badge(),
        }
    }

    /// Either side may move, from any shown position; moving from an earlier one replaces
    /// what followed it. Premoves that are due then play. A game the rules already ended (a
    /// repetition, say) takes no more moves.
    fn play_move(&mut self, mv: Move, cx: &mut Context<Self>) {
        if self.play_with_sound(mv, cx) {
            self.clear_review();
            self.run_premoves(cx);
            self.after_position_change(cx);
        }
        cx.notify();
    }

    /// Plays queued premoves while each is legal; one that is not drops its side's queue, and
    /// a game that ends drops them all.
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

    /// Reads the field: a valid FEN starts a fresh analysis, an invalid one says why, and an
    /// empty field does nothing.
    fn load_fen(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.fen_input.read(cx).value();
        if text.trim().is_empty() {
            self.fen_error = None;
            cx.notify();
            return;
        }
        match text.trim().parse::<Position>() {
            Ok(position) => {
                self.history = GameHistory::from_position(position);
                self.clear_review();
                self.fen_error = None;
                self.fen_input
                    .update(cx, |input, cx| input.set_value("", window, cx));
                self.after_navigation(cx);
            }
            Err(error) => {
                self.fen_error = Some(format!("Not a valid FEN: {error}"));
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

    /// The best move as an arrow, while the engine is switched on and the move is legal on the
    /// board shown; switching off removes it at once.
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

    /// The eval bar on the left of the board, both a little shorter than the space they have.
    fn board_column(&self, cx: &App) -> Div {
        h_flex()
            .h(relative(0.9))
            .flex_shrink_0()
            .items_stretch()
            .gap_2()
            .child(EvalBar::new(self.eval(), self.board.read(cx).flipped()))
            .child(div().h_full().child(self.board.clone()))
    }

    /// The engine, the history, a FEN field, and the shown position's FEN.
    fn side_column(&self, page: WeakEntity<Self>) -> Div {
        v_flex()
            .w_80()
            .flex_shrink_0()
            .gap_4()
            .children(self.review_panel())
            .child(EnginePanel::new(
                self.enabled,
                self.engine_status(),
                self.line_views(),
                toggle_engine(page.clone()),
            ))
            .child(
                NavigationBar::new(
                    self.history.can_go_back(),
                    self.history.can_go_forward(),
                    step_history(page.clone()),
                )
                .on_hold(hold_step(page.clone())),
            )
            .child(
                MoveList::new(
                    self.history.sans().to_vec(),
                    self.history.viewing(),
                    select_move(page.clone()),
                )
                .classes(self.review_classes()),
            )
            .child(FenInput::new(
                self.fen_input.clone(),
                self.fen_error.clone(),
                load_fen(page),
            ))
            .child(PositionPanel::new(*self.history.shown()))
    }

    /// Starts a review of the game on the board with an engine of its own; live analysis
    /// pauses until it is done.
    fn start_review(&mut self, cx: &mut Context<Self>) {
        if self.history.sans().is_empty() || self.reviewer.is_some() {
            return;
        }
        let Some(path) = locate_stockfish() else {
            self.problem = Some(EngineProblem::NotFound);
            cx.notify();
            return;
        };
        let Ok((engine, events)) = Engine::spawn(&path) else {
            self.problem = Some(EngineProblem::Stopped);
            cx.notify();
            return;
        };
        let settings = EngineSettings {
            threads: self.settings.threads,
            hash_mb: REVIEW_HASH_MB,
            depth: Some(REVIEW_DEPTH),
        };
        // The engine takes seconds; the opening table is built meanwhile, off this thread.
        cx.background_spawn(async { warm_opening_book() }).detach();
        let (queue, first) = SearchQueue::new(settings);
        self.reviewer = Some(Reviewer {
            engine,
            queue,
            run: ReviewRun::new(self.history.game()),
            lines: Vec::new(),
            _task: listen_review(events, cx),
        });
        self.review = None;
        self.stop_search();
        self.send_review(&first);
        cx.notify();
    }

    fn on_review_event(&mut self, event: EngineEvent, cx: &mut Context<Self>) {
        match event {
            EngineEvent::Message(message) => self.on_review_message(message, cx),
            EngineEvent::Stopped => self.review_failed(cx),
        }
        cx.notify();
    }

    fn on_review_message(&mut self, message: UciMessage, cx: &mut Context<Self>) {
        let Some(reviewer) = self.reviewer.as_mut() else {
            return;
        };
        match message {
            UciMessage::UciOk => {
                let commands = reviewer.queue.on_uciok();
                self.send_review(&commands);
            }
            UciMessage::ReadyOk => {
                let commands = reviewer.queue.on_readyok();
                self.send_review(&commands);
                self.review_next(cx);
            }
            UciMessage::Info(info) => {
                if reviewer.queue.accepts_info() {
                    reviewer.lines.retain(|line| line.multipv != info.multipv);
                    reviewer.lines.push(info);
                }
            }
            UciMessage::BestMove(_) => {
                let commands = reviewer.queue.on_bestmove();
                let lines = std::mem::take(&mut reviewer.lines);
                reviewer.run.store(&lines);
                self.send_review(&commands);
                self.review_next(cx);
            }
            UciMessage::Other => {}
        }
    }

    /// Searches the next position, or finishes the review once every one has its value.
    fn review_next(&mut self, cx: &mut Context<Self>) {
        let Some(reviewer) = self.reviewer.as_mut() else {
            return;
        };
        match reviewer.run.next_position() {
            Some((index, fen)) => {
                let commands = reviewer.queue.request(fen);
                self.send_review(&commands);
                self.show_position(index, cx);
            }
            None => {
                self.review = reviewer.run.finish();
                self.reviewer = None;
                play_sound(Sound::Notify, cx);
                self.show_position(0, cx);
            }
        }
    }

    /// Shows the game's position at `index`, 0 being the start: the review walks the board
    /// through the game as it goes, each move with its sound as when stepping through it, and
    /// starts it over from the beginning once it is ready.
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

    fn send_review(&mut self, commands: &[Command]) {
        let Some(reviewer) = self.reviewer.as_mut() else {
            return;
        };
        if commands
            .iter()
            .any(|command| reviewer.engine.send(command).is_err())
        {
            self.reviewer = None;
            self.problem = Some(EngineProblem::Stopped);
        }
    }

    /// The review's engine died: the review is dropped and the engine panel says so.
    fn review_failed(&mut self, cx: &mut Context<Self>) {
        self.reviewer = None;
        self.problem = Some(EngineProblem::Stopped);
        self.sync_engine(cx);
    }

    /// A review belongs to one game: any change to the game, or a new one, drops it and any
    /// review still running.
    fn clear_review(&mut self) {
        self.review = None;
        self.reviewer = None;
    }

    /// Each move's class, once reviewed.
    fn review_classes(&self) -> Vec<MoveClass> {
        self.review.as_ref().map_or_else(Vec::new, |review| {
            review.moves.iter().map(|reviewed| reviewed.class).collect()
        })
    }

    /// The engine's choice where the move that led here was not it, as chess.com shows it. The
    /// live engine's arrow gives way meanwhile, so the board has one arrow to read.
    fn best_arrow(&self) -> Option<(Square, Square)> {
        let (_, better) = self.better_move()?;
        Some((better.from, better.to))
    }

    /// What the engine preferred to the move that led to the shown position, with that move's
    /// index: for any move but the best, a Book move or a Forced one.
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

    fn review_panel(&self) -> Option<ReviewPanel> {
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
            .filter(|(_, white, black)| white + black > 0)
            .collect();
        Some(ReviewPanel::new(
            (review.accuracy, review.ratings),
            counts,
            self.shown_verdict(),
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

    /// Review, or how far a running review has got.
    fn review_button(&self, cx: &mut Context<Self>) -> Button {
        let label = match &self.reviewer {
            Some(reviewer) => {
                let (done, total) = reviewer.run.progress();
                format!("Reviewing {done}/{total}")
            }
            None => "Review".to_owned(),
        };
        Button::new("review-game")
            .primary()
            .icon(IconName::Sparkles)
            .label(label)
            .loading(self.reviewer.is_some())
            .disabled(self.reviewer.is_some())
            .on_click(cx.listener(|page, _, _, cx| page.start_review(cx)))
    }

    fn header(&self, cx: &mut Context<Self>) -> Div {
        h_flex()
            .justify_between()
            .child(Label::new("Analysis").text_2xl())
            .child(
                h_flex()
                    .gap_2()
                    .when(!self.history.sans().is_empty(), |this| {
                        this.child(self.review_button(cx))
                    })
                    .child(flip_button(cx))
                    .child(reset_button(cx)),
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
            .p_6()
            .gap_4()
            .child(self.header(cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_start()
                    .justify_center()
                    .gap_6()
                    .child(self.board_column(cx))
                    .child(self.side_column(page)),
            )
    }
}

/// Reads the review's engine output until it ends or the page is gone.
fn listen_review(
    events: gpui_kit::base::async_util::Receiver<EngineEvent>,
    cx: &mut Context<AnalysisPage>,
) -> Task<()> {
    cx.spawn(async move |page, cx| {
        while let Ok(event) = events.recv().await {
            if page
                .update(cx, |page, cx| page.on_review_event(event, cx))
                .is_err()
            {
                break;
            }
        }
    })
}

fn flip_button(cx: &mut Context<AnalysisPage>) -> Button {
    Button::new("analysis-flip-board")
        .ghost()
        .icon(IconName::ArrowUpDown)
        .tooltip("Flip board")
        .on_click(cx.listener(|page, _, _, cx| page.flip(cx)))
}

fn reset_button(cx: &mut Context<AnalysisPage>) -> Button {
    Button::new("reset-board")
        .outline()
        .icon(IconName::RefreshCw)
        .label("Reset board")
        .on_click(cx.listener(|page, _, _, cx| page.reset_board(cx)))
}

/// Enter in the FEN field loads it, like the Load button.
fn on_fen_event(
    page: &mut AnalysisPage,
    _: &Entity<InputState>,
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

    use gpui_kit::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext};

    use chess_domain::{Game, Move};

    use super::AnalysisPage;
    use crate::{
        clients::engine::locate_stockfish,
        enums::{analysis::MoveClass, history::NavStep, sound::Sound},
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
            page.load_game(&fools_mate(), 0, cx);
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
            page.load_game(&fools_mate(), 4, cx);
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
        assert!(
            played
                .iter()
                .filter(|&&sound| sound == Sound::MoveSelf)
                .count()
                >= 3,
            "f3, e5 and g4 sound as the review reaches them: {played:?}"
        );
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
}
