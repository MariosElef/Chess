use std::rc::Rc;

use chess_domain::{
    Game, GameStatus, Move, PieceKind, Position, Square,
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

use super::{
    engine_panel::{EnginePanel, EngineStatus, LineView, numbered_line},
    eval_bar::EvalBar,
    fen_input::FenInput,
};
use crate::{
    board::{BoardGrid, BoardInteraction, PromotionPrompt},
    engine::{Engine, EngineEvent, EngineSettings, SearchQueue, locate_stockfish},
    history::{
        CONTEXT, FirstPosition, GameHistory, LivePosition, MoveList, NavStep, NavigationBar,
        NextPosition, PositionPanel, PreviousPosition,
    },
};

/// How many lines the engine panel shows, matching the engine's MultiPV.
const LINES: usize = 3;
/// How many moves of each line are written out.
const LINE_LENGTH: usize = 12;

/// Why the engine is not running although it is switched on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EngineProblem {
    NotFound,
    Stopped,
}

/// A board to move freely on, with Stockfish analysing whatever position it shows.
pub struct AnalysisPage {
    history: GameHistory,
    interaction: BoardInteraction,
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
        Self {
            history: GameHistory::new(),
            interaction: BoardInteraction::default(),
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
        self.after_position_change(cx);
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
        self.after_position_change(cx);
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
        self.interaction.clear();
        self.sync_engine(cx);
        cx.notify();
    }

    fn click_square(&mut self, square: Square, cx: &mut Context<Self>) {
        let mv = self.interaction.click(self.history.shown(), square, true);
        self.play_if_any(mv, cx);
    }

    fn drop_piece(&mut self, from: Square, to: Square, cx: &mut Context<Self>) {
        let mv = self.interaction.drop(self.history.shown(), from, to, true);
        self.play_if_any(mv, cx);
    }

    fn promote(&mut self, kind: PieceKind, cx: &mut Context<Self>) {
        let mv = self.interaction.promote(kind);
        self.play_if_any(mv, cx);
    }

    /// Either side may move, from any shown position; moving from an earlier one replaces
    /// what followed it. A game the rules already ended (a repetition, say) takes no more moves.
    fn play_if_any(&mut self, mv: Option<Move>, cx: &mut Context<Self>) {
        if let Some(mv) = mv
            && self.history.play_from_shown(mv).is_ok()
        {
            self.after_position_change(cx);
        }
        cx.notify();
    }

    fn cancel_promotion(&mut self, cx: &mut Context<Self>) {
        self.interaction.cancel_promotion();
        cx.notify();
    }

    fn right_down(&mut self, square: Square) {
        self.interaction.right_down(square);
    }

    fn right_up(&mut self, square: Square, cx: &mut Context<Self>) {
        if self.interaction.right_up(square) {
            cx.notify();
        }
    }

    fn flip(&mut self, cx: &mut Context<Self>) {
        self.interaction.flip();
        cx.notify();
    }

    fn navigate(&mut self, step: NavStep, cx: &mut Context<Self>) {
        self.history.step(step);
        self.after_position_change(cx);
    }

    fn select_move(&mut self, index: usize, cx: &mut Context<Self>) {
        self.history.jump_to_move(index);
        self.after_position_change(cx);
    }

    fn previous_position(&mut self, _: &PreviousPosition, _: &mut Window, cx: &mut Context<Self>) {
        self.navigate(NavStep::Back, cx);
    }

    fn next_position(&mut self, _: &NextPosition, _: &mut Window, cx: &mut Context<Self>) {
        self.navigate(NavStep::Forward, cx);
    }

    fn first_position(&mut self, _: &FirstPosition, _: &mut Window, cx: &mut Context<Self>) {
        self.navigate(NavStep::Start, cx);
    }

    fn live_position(&mut self, _: &LivePosition, _: &mut Window, cx: &mut Context<Self>) {
        self.navigate(NavStep::Live, cx);
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
                self.after_position_change(cx);
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

    /// The best move as an arrow, when it is legal on the board shown.
    fn engine_arrow(&self) -> Option<(Square, Square)> {
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

    fn board(&self, page: WeakEntity<Self>) -> BoardGrid {
        let shown = self.history.shown();
        let draggable = self
            .interaction
            .accepts_moves(true)
            .then(|| shown.side_to_move());
        self.interaction
            .grid(shown, self.history.shown_last_move(), draggable)
            .engine_arrow(self.engine_arrow())
            .promotion(self.promotion_prompt(page.clone()))
            .on_right_button(right_down(page.clone()), right_up(page.clone()))
            .on_square_click(click_square(page.clone()))
            .on_piece_drop(drop_piece(page))
    }

    fn promotion_prompt(&self, page: WeakEntity<Self>) -> Option<PromotionPrompt> {
        self.interaction.is_promoting().then(|| PromotionPrompt {
            side: self.history.shown().side_to_move(),
            on_pick: Rc::new(pick_promotion(page.clone())),
            on_cancel: Rc::new(cancel_promotion(page)),
        })
    }

    /// The eval bar on the left of the board, both a little shorter than the space they have.
    fn board_column(&self, page: WeakEntity<Self>) -> Div {
        h_flex()
            .h(relative(0.9))
            .flex_shrink_0()
            .items_stretch()
            .gap_2()
            .child(EvalBar::new(self.eval(), self.interaction.flipped()))
            .child(div().h_full().child(self.board(page)))
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

impl Render for AnalysisPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = cx.entity().downgrade();
        v_flex()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::previous_position))
            .on_action(cx.listener(Self::next_position))
            .on_action(cx.listener(Self::first_position))
            .on_action(cx.listener(Self::live_position))
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
                    .child(self.board_column(page.clone()))
                    .child(self.side_column(page)),
            )
    }
}

/// One engine line in the panel: its evaluation for White, then its moves numbered.
fn line_view(info: &SearchInfo, analysed: &Position) -> LineView {
    let moves = &info.pv[..info.pv.len().min(LINE_LENGTH)];
    let sans = analysed.san_line(moves);
    LineView {
        eval: Eval::from_uci(info.score, analysed.side_to_move()),
        moves: numbered_line(
            analysed.counters().fullmove_number,
            analysed.side_to_move(),
            &sans,
        ),
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

fn step_history(
    page: WeakEntity<AnalysisPage>,
) -> impl Fn(NavStep, &mut Window, &mut App) + 'static {
    move |step, _, cx| {
        let _ = page.update(cx, |page, cx| page.navigate(step, cx));
    }
}

fn select_move(page: WeakEntity<AnalysisPage>) -> impl Fn(usize, &mut Window, &mut App) + 'static {
    move |index, _, cx| {
        let _ = page.update(cx, |page, cx| page.select_move(index, cx));
    }
}

fn click_square(
    page: WeakEntity<AnalysisPage>,
) -> impl Fn(Square, &mut Window, &mut App) + 'static {
    move |square, _, cx| {
        let _ = page.update(cx, |page, cx| page.click_square(square, cx));
    }
}

fn right_down(page: WeakEntity<AnalysisPage>) -> impl Fn(Square, &mut Window, &mut App) + 'static {
    move |square, _, cx| {
        let _ = page.update(cx, |page, _| page.right_down(square));
    }
}

fn right_up(page: WeakEntity<AnalysisPage>) -> impl Fn(Square, &mut Window, &mut App) + 'static {
    move |square, _, cx| {
        let _ = page.update(cx, |page, cx| page.right_up(square, cx));
    }
}

fn drop_piece(
    page: WeakEntity<AnalysisPage>,
) -> impl Fn(Square, Square, &mut Window, &mut App) + 'static {
    move |from, to, _, cx| {
        let _ = page.update(cx, |page, cx| page.drop_piece(from, to, cx));
    }
}

fn pick_promotion(
    page: WeakEntity<AnalysisPage>,
) -> impl Fn(PieceKind, &mut Window, &mut App) + 'static {
    move |kind, _, cx| {
        let _ = page.update(cx, |page, cx| page.promote(kind, cx));
    }
}

fn cancel_promotion(page: WeakEntity<AnalysisPage>) -> impl Fn(&mut Window, &mut App) + 'static {
    move |_, cx| {
        let _ = page.update(cx, |page, cx| page.cancel_promotion(cx));
    }
}
