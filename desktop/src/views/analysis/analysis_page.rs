use chess_domain::{
    Game, GameStatus, Move, Position, Square,
    engine::{Command, Eval, SearchInfo, UciMessage},
};
use gpui_kit::{
    assets::IconName,
    component::{
        button::{Button, ButtonVariants},
        h_flex,
        input::{InputEvent, InputState},
        label::Label,
        v_flex,
    },
    *,
};

use crate::{
    clients::engine::{Engine, locate_stockfish},
    components::{
        analysis::{EnginePanel, EvalBar, FenInput},
        history::{MoveList, NavigationBar, PositionPanel},
    },
    enums::{analysis::EngineStatus, engine::EngineEvent, history::NavStep, sound::Sound},
    models::{
        analysis::LineView,
        board::{MoveMade, ShownPosition},
        engine::EngineSettings,
    },
    services::{analysis::line_view, engine::SearchQueue, history::GameHistory},
    views::{
        board::BoardView,
        history::{HistoryNavigation, select_move, step_history, with_history_keys},
        sound::play_move_sound,
    },
};

/// How many lines the engine panel shows, matching the engine's MultiPV.
const LINES: usize = 3;

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

    /// Searches the shown position while switched on and visible; otherwise stops searching.
    fn sync_engine(&mut self, cx: &mut Context<Self>) {
        if self.enabled && self.shown && self.problem.is_none() {
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
            engine_arrow: self.engine_arrow(),
        }
    }

    /// Either side may move, from any shown position; moving from an earlier one replaces
    /// what followed it. Premoves that are due then play. A game the rules already ended (a
    /// repetition, say) takes no more moves.
    fn play_move(&mut self, mv: Move, cx: &mut Context<Self>) {
        if self.play_with_sound(mv, cx) {
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
            .child(EnginePanel::new(
                self.enabled,
                self.engine_status(),
                self.line_views(),
                toggle_engine(page.clone()),
            ))
            .child(NavigationBar::new(
                self.history.can_go_back(),
                self.history.can_go_forward(),
                step_history(page.clone()),
            ))
            .child(MoveList::new(
                self.history.sans().to_vec(),
                self.history.viewing(),
                select_move(page.clone()),
            ))
            .child(FenInput::new(
                self.fen_input.clone(),
                self.fen_error.clone(),
                load_fen(page),
            ))
            .child(PositionPanel::new(*self.history.shown()))
    }

    fn header(&self, cx: &mut Context<Self>) -> Div {
        h_flex()
            .justify_between()
            .child(Label::new("Analysis").text_2xl())
            .child(
                h_flex()
                    .gap_2()
                    .child(flip_button(cx))
                    .child(reset_button(cx)),
            )
    }
}

impl HistoryNavigation for AnalysisPage {
    fn navigate(&mut self, step: NavStep, cx: &mut Context<Self>) {
        self.history.step(step);
        self.after_navigation(cx);
    }

    fn select_move(&mut self, index: usize, cx: &mut Context<Self>) {
        self.history.jump_to_move(index);
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
    use gpui_kit::{Entity, TestAppContext, VisualTestContext};

    use super::AnalysisPage;
    use crate::{
        enums::sound::Sound,
        models::engine::EngineSettings,
        testing::{click, drag},
        views::sound::{played_sounds, record_sounds},
    };

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
}
