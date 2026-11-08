use std::rc::Rc;

use chess_domain::{
    Board, Game, GameStatus, Move, PieceKind, Position, Side, Square,
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
        board::{BoardGrid, PromotionPrompt},
        history::{MoveList, NavigationBar, PositionPanel},
    },
    enums::{analysis::EngineStatus, board::ClickMode, engine::EngineEvent, history::NavStep},
    models::{analysis::LineView, engine::EngineSettings},
    services::{
        analysis::line_view,
        board::{BoardInteraction, PremoveQueue, click_mode},
        engine::SearchQueue,
        history::GameHistory,
    },
    views::history::{CONTEXT, FirstPosition, LivePosition, NextPosition, PreviousPosition},
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
    interaction: BoardInteraction,
    /// Moves queued for the side not to move, played after the side to move's move.
    premoves: PremoveQueue,
    /// The open promotion picker is for a premove, not a move.
    pending_premove: bool,
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
            premoves: PremoveQueue::default(),
            pending_premove: false,
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
        self.premoves.clear_all();
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
        self.premoves.clear_all();
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
        match self.click_mode(square) {
            ClickMode::Move => {
                let mv = self.interaction.click(self.history.shown(), square, true);
                self.play_if_any(mv, cx);
            }
            ClickMode::Premove => {
                let mv =
                    self.interaction
                        .premove_click(&self.preview(), self.waiting_side(), square);
                self.queue_if_any(mv, cx);
            }
        }
    }

    fn drop_piece(&mut self, from: Square, to: Square, cx: &mut Context<Self>) {
        match self.click_mode(from) {
            ClickMode::Move => {
                let mv = self.interaction.drop(self.history.shown(), from, to, true);
                self.play_if_any(mv, cx);
            }
            ClickMode::Premove => {
                let mv =
                    self.interaction
                        .premove_drop(&self.preview(), self.waiting_side(), from, to);
                self.queue_if_any(mv, cx);
            }
        }
    }

    fn promote(&mut self, kind: PieceKind, cx: &mut Context<Self>) {
        let mv = self.interaction.promote(kind);
        if self.pending_premove {
            self.queue_if_any(mv, cx);
        } else {
            self.play_if_any(mv, cx);
        }
    }

    fn click_mode(&self, square: Square) -> ClickMode {
        click_mode(
            &self.preview(),
            self.waiting_side(),
            self.interaction.selected(),
            square,
        )
    }

    /// The side not to move in the shown position: the one that may queue premoves.
    fn waiting_side(&self) -> Side {
        self.history.shown().side_to_move().opponent()
    }

    /// The shown pieces as the queued premoves leave them.
    fn preview(&self) -> Board {
        let shown = self.history.shown();
        self.premoves.preview(shown.board(), shown.side_to_move())
    }

    /// Either side may move, from any shown position; moving from an earlier one replaces
    /// what followed it. Premoves that are due then play. A game the rules already ended (a
    /// repetition, say) takes no more moves.
    fn play_if_any(&mut self, mv: Option<Move>, cx: &mut Context<Self>) {
        self.pending_premove = false;
        if let Some(mv) = mv
            && self.history.play_from_shown(mv).is_ok()
        {
            self.run_premoves();
            self.after_position_change(cx);
        }
        cx.notify();
    }

    /// Queues the premove, or remembers that the promotion picker now open is for one.
    fn queue_if_any(&mut self, mv: Option<Move>, cx: &mut Context<Self>) {
        self.pending_premove = self.interaction.is_promoting();
        if let Some(mv) = mv {
            self.premoves.queue(self.waiting_side(), mv);
        }
        cx.notify();
    }

    /// Plays queued premoves while each is legal; one that is not drops its side's queue, and
    /// a game that ends drops them all.
    fn run_premoves(&mut self) {
        while let Some(mv) = self.premoves.next_playable(self.history.shown()) {
            if self.history.play_from_shown(mv).is_err() {
                self.premoves.clear_all();
                return;
            }
        }
        if self.history.game().is_over() {
            self.premoves.clear_all();
        }
    }

    fn cancel_promotion(&mut self, cx: &mut Context<Self>) {
        self.interaction.cancel_promotion();
        cx.notify();
    }

    /// A right-click with premoves queued cancels them all and draws nothing.
    fn right_down(&mut self, square: Square, cx: &mut Context<Self>) {
        if self.premoves.is_empty() {
            self.interaction.right_down(square);
        } else {
            self.premoves.clear_all();
            cx.notify();
        }
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
        self.premoves.clear_all();
        self.after_position_change(cx);
    }

    fn select_move(&mut self, index: usize, cx: &mut Context<Self>) {
        self.history.jump_to_move(index);
        self.premoves.clear_all();
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
                self.premoves.clear_all();
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

    fn board(&self, page: WeakEntity<Self>) -> BoardGrid {
        let shown = self.history.shown();
        let board = self.preview();
        let draggable = if self.interaction.accepts_moves(true) {
            vec![Side::White, Side::Black]
        } else {
            Vec::new()
        };
        BoardGrid::for_interaction(
            &self.interaction,
            shown,
            &board,
            self.shown_targets(&board),
            self.history.shown_last_move(),
            draggable,
        )
        .premoves(self.premoves.pairs())
        .engine_arrow(self.engine_arrow())
        .promotion(self.promotion_prompt(page.clone()))
        .on_right_button(right_down(page.clone()), right_up(page.clone()))
        .on_square_click(click_square(page.clone()))
        .on_piece_drop(drop_piece(page))
    }

    /// The dots: where the selected piece may move, or may premove on the preview.
    fn shown_targets(&self, board: &Board) -> Vec<Square> {
        match self.interaction.selected() {
            Some(selected) if self.click_mode(selected) == ClickMode::Premove => {
                self.interaction.premove_targets(board)
            }
            Some(_) | None => self.interaction.legal_targets(self.history.shown()),
        }
    }

    /// The pawn's side: the mover's, or the waiting side's for a premove.
    fn promoting_side(&self) -> Side {
        let mover = self.history.shown().side_to_move();
        if self.pending_premove {
            mover.opponent()
        } else {
            mover
        }
    }

    fn promotion_prompt(&self, page: WeakEntity<Self>) -> Option<PromotionPrompt> {
        self.interaction.is_promoting().then(|| PromotionPrompt {
            side: self.promoting_side(),
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
        let _ = page.update(cx, |page, cx| page.right_down(square, cx));
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

#[cfg(test)]
mod tests {
    use gpui_kit::{Entity, TestAppContext, VisualTestContext};

    use super::AnalysisPage;
    use crate::{
        models::engine::EngineSettings,
        views::testing::{click, drag},
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
        page.update(cx, |page, _| {
            assert_eq!(page.history.sans(), ["f3", "e5", "g4", "Qh4#"]);
            assert!(
                page.premoves.is_empty(),
                "a6 is dropped once the game is over"
            );
        });
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
        page.update(cx, |page, _| {
            assert_eq!(page.history.sans(), ["d4", "Nf6"]);
            assert!(!page.premoves.is_empty(), "Ng4 waits for White's next move");
        });
        drag(cx, "square-c2", "square-c4");
        page.update(cx, |page, _| {
            assert_eq!(page.history.sans(), ["d4", "Nf6", "c4", "Ng4"]);
        });
    }
}
