use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use chess_domain::{Board, Clock, Move, PieceKind, Side, Square, TimeControl};
use gpui_kit::{
    assets::IconName,
    component::{
        Disableable as _,
        button::{Button, ButtonVariants},
        h_flex,
        label::Label,
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

/// How often the running clock is redrawn and checked for a flag.
const TICK: Duration = Duration::from_millis(100);
/// What a premove costs on the clock, as on chess.com; a side with less than this cannot play one.
const PREMOVE_COST: Duration = Duration::from_millis(100);

/// The page shows the new-game menu until a game starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Setup,
    Playing,
}
use crate::{
    components::{
        board::{BoardGrid, PromotionPrompt},
        history::{MoveList, NavigationBar, PositionPanel},
        play::{CapturedRow, ClockFace, GameSetup},
    },
    enums::{board::ClickMode, history::NavStep},
    models::play::AnalyzeGame,
    services::{
        board::{BoardInteraction, PremoveQueue, click_mode},
        history::GameHistory,
        play::{DEFAULT_TIME_CONTROL, status_text},
    },
    views::history::{CONTEXT, FirstPosition, LivePosition, NextPosition, PreviousPosition},
};

/// The game being played, the position on the board, and what the player is in the middle of.
pub struct PlayPage {
    stage: Stage,
    /// The time control picked in the menu; `None` is no clock.
    setup_choice: Option<TimeControl>,
    history: GameHistory,
    clock: Option<Clock>,
    /// When the side to move's turn began; `None` before the first move and once the game is over.
    turn_started: Option<Instant>,
    /// Redraws the running clock; dropping it stops the ticking.
    _ticker: Option<Task<()>>,
    /// The picked-up piece, promotion choice, drawings and board orientation.
    interaction: BoardInteraction,
    /// Moves each side has queued for when its turn comes.
    premoves: PremoveQueue,
    /// The open promotion picker is for a premove, not a move.
    pending_premove: bool,
    /// Keyboard focus, so the arrow keys reach this page.
    focus_handle: FocusHandle,
}

impl PlayPage {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            stage: Stage::Setup,
            setup_choice: DEFAULT_TIME_CONTROL,
            history: GameHistory::new(),
            clock: None,
            turn_started: None,
            _ticker: None,
            interaction: BoardInteraction::default(),
            premoves: PremoveQueue::default(),
            pending_premove: false,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus_handle.focus(window, cx);
    }

    /// Moves can be made only on the live position of a game still going.
    fn can_play(&self) -> bool {
        self.history.is_live() && !self.history.game().is_over()
    }

    fn click_square(&mut self, square: Square, cx: &mut Context<Self>) {
        match self.click_mode(square) {
            ClickMode::Move => {
                let mv = self
                    .interaction
                    .click(self.history.live(), square, self.can_play());
                self.play_if_any(mv);
            }
            ClickMode::Premove => {
                let mv =
                    self.interaction
                        .premove_click(&self.preview(), self.waiting_side(), square);
                self.queue_if_any(mv);
            }
        }
        cx.notify();
    }

    fn drop_piece(&mut self, from: Square, to: Square, cx: &mut Context<Self>) {
        match self.click_mode(from) {
            ClickMode::Move => {
                let mv = self
                    .interaction
                    .drop(self.history.live(), from, to, self.can_play());
                self.play_if_any(mv);
            }
            ClickMode::Premove => {
                let mv =
                    self.interaction
                        .premove_drop(&self.preview(), self.waiting_side(), from, to);
                self.queue_if_any(mv);
            }
        }
        cx.notify();
    }

    /// Nothing is premoved on an earlier position or once the game is over.
    fn click_mode(&self, square: Square) -> ClickMode {
        if !self.can_play() {
            return ClickMode::Move;
        }
        click_mode(
            &self.preview(),
            self.waiting_side(),
            self.interaction.selected(),
            square,
        )
    }

    /// The side not to move: the one that may queue premoves.
    fn waiting_side(&self) -> Side {
        self.side_to_move().opponent()
    }

    /// The live pieces as the queued premoves leave them.
    fn preview(&self) -> Board {
        self.premoves
            .preview(self.history.live().board(), self.side_to_move())
    }

    fn play_if_any(&mut self, mv: Option<Move>) {
        self.pending_premove = false;
        if let Some(mv) = mv {
            self.play(mv);
        }
    }

    /// Queues the premove, or remembers that the promotion picker now open is for one.
    fn queue_if_any(&mut self, mv: Option<Move>) {
        self.pending_premove = self.interaction.is_promoting();
        if let Some(mv) = mv {
            self.premoves.queue(self.waiting_side(), mv);
        }
    }

    fn choose_time(&mut self, choice: Option<TimeControl>, cx: &mut Context<Self>) {
        self.setup_choice = choice;
        cx.notify();
    }

    /// Leaves the menu with a fresh game and, for a timed game, a ticking clock.
    fn start_game(&mut self, cx: &mut Context<Self>) {
        self.history = GameHistory::new();
        self.clock = self.setup_choice.map(Clock::new);
        self.turn_started = None;
        self._ticker = self.clock.is_some().then(|| start_ticker(cx));
        self.stage = Stage::Playing;
        self.end_interaction(cx);
    }

    /// Plays a move picked from the legal ones; the mover's clock is charged, and the
    /// opponent's starts unless the move ended the game. Then any premoves that are due play.
    fn play(&mut self, mv: Move) {
        let mover = self.side_to_move();
        self.charge_clock(mover, Clock::finish_turn);
        self.record_move(mv);
        self.run_premoves();
    }

    fn record_move(&mut self, mv: Move) {
        self.history
            .play(mv)
            .expect("moves come from the legal ones");
        self.turn_started = (!self.history.game().is_over()).then(Instant::now);
    }

    /// Plays the side to move's queued premoves while each is legal, then the other side's,
    /// and so on; the first one that is not drops the rest of that side's queue, and a game
    /// that ends drops them all.
    fn run_premoves(&mut self) {
        loop {
            let side = self.side_to_move();
            if self.history.game().is_over() {
                self.premoves.clear_all();
                return;
            }
            if !self.clock_allows_premove(side) {
                self.premoves.clear(side);
                return;
            }
            let Some(mv) = self.premoves.next_playable(self.history.live()) else {
                return;
            };
            self.play_premove(side, mv);
        }
    }

    /// A premove needs more than its cost left on the clock.
    fn clock_allows_premove(&self, side: Side) -> bool {
        self.clock
            .is_none_or(|clock| clock.remaining(side, Duration::ZERO) > PREMOVE_COST)
    }

    /// A premove is played at once for a fixed cost instead of the time the turn took.
    fn play_premove(&mut self, side: Side, mv: Move) {
        if let Some(clock) = self.clock.as_mut() {
            clock.finish_turn(side, PREMOVE_COST);
        }
        self.record_move(mv);
    }

    /// Applies the running turn's time to `side` with `charge`, if a clock is running.
    fn charge_clock(&mut self, side: Side, charge: fn(&mut Clock, Side, Duration)) {
        if let (Some(clock), Some(started)) = (self.clock.as_mut(), self.turn_started) {
            charge(clock, side, started.elapsed());
        }
    }

    /// The game ended mid-turn: the side to move's thinking time is taken, and the clocks freeze.
    fn stop_clock(&mut self) {
        self.charge_clock(self.side_to_move(), Clock::spend);
        self.turn_started = None;
    }

    /// Redraws the clocks and ends the game when the side to move's time runs out.
    /// Returns whether the ticker should keep going.
    fn tick(&mut self, cx: &mut Context<Self>) -> bool {
        if self.history.game().is_over() {
            return false;
        }
        if self.is_flagged() {
            self.stop_clock();
            self.history
                .flag(self.side_to_move())
                .expect("the game was still going");
            self.end_interaction(cx);
            return false;
        }
        cx.notify();
        true
    }

    fn is_flagged(&self) -> bool {
        match (self.clock, self.turn_started) {
            (Some(clock), Some(started)) => {
                clock.is_flagged(self.side_to_move(), started.elapsed())
            }
            _ => false,
        }
    }

    /// The side whose clock is running, if any.
    fn running_side(&self) -> Option<Side> {
        self.turn_started.map(|_| self.side_to_move())
    }

    /// Time left for `side` right now, or `None` in a game without a clock.
    fn time_left(&self, side: Side) -> Option<Duration> {
        let elapsed = match (self.running_side(), self.turn_started) {
            (Some(running), Some(started)) if running == side => started.elapsed(),
            _ => Duration::ZERO,
        };
        self.clock.map(|clock| clock.remaining(side, elapsed))
    }

    fn promote(&mut self, kind: PieceKind, cx: &mut Context<Self>) {
        let mv = self.interaction.promote(kind);
        if self.pending_premove {
            self.queue_if_any(mv);
        } else {
            self.play_if_any(mv);
        }
        cx.notify();
    }

    fn cancel_promotion(&mut self, cx: &mut Context<Self>) {
        self.interaction.cancel_promotion();
        cx.notify();
    }

    /// A right-click with premoves queued cancels them all, as on chess.com, and draws nothing.
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

    fn side_to_move(&self) -> Side {
        self.history.live().side_to_move()
    }

    fn resign(&mut self, cx: &mut Context<Self>) {
        self.stop_clock();
        self.history
            .resign(self.side_to_move())
            .expect("the button only shows while the game is on");
        self.end_interaction(cx);
    }

    fn offer_draw(&mut self, cx: &mut Context<Self>) {
        self.history
            .offer_draw(self.side_to_move())
            .expect("the button only shows while the game is on");
        cx.notify();
    }

    fn accept_draw(&mut self, cx: &mut Context<Self>) {
        self.stop_clock();
        self.history
            .accept_draw(self.side_to_move())
            .expect("the button only shows for an offer made to this side");
        self.end_interaction(cx);
    }

    fn decline_draw(&mut self, cx: &mut Context<Self>) {
        self.history
            .decline_draw(self.side_to_move())
            .expect("the button only shows for an offer made to this side");
        cx.notify();
    }

    fn flip(&mut self, cx: &mut Context<Self>) {
        self.interaction.flip();
        cx.notify();
    }

    /// Hands the game to the shell, which opens it on the analysis board.
    fn analyze(&mut self, cx: &mut Context<Self>) {
        cx.emit(AnalyzeGame {
            game: self.history.game().clone(),
            viewing: self.history.viewing(),
        });
    }

    /// Back to the menu, keeping the last time control picked.
    fn new_game(&mut self, cx: &mut Context<Self>) {
        self._ticker = None;
        self.turn_started = None;
        self.stage = Stage::Setup;
        self.end_interaction(cx);
    }

    /// Puts down a picked-up piece, closes the promotion picker and wipes the drawings.
    fn end_interaction(&mut self, cx: &mut Context<Self>) {
        self.interaction.clear();
        self.premoves.clear_all();
        self.pending_premove = false;
        cx.notify();
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

    fn select_move(&mut self, index: usize, cx: &mut Context<Self>) {
        self.history.jump_to_move(index);
        self.end_interaction(cx);
    }

    /// Moves the board through the history; the picked-up piece is put down first.
    fn navigate(&mut self, step: NavStep, cx: &mut Context<Self>) {
        self.history.step(step);
        self.end_interaction(cx);
    }

    /// An offer the side to move has received from the other side.
    fn has_offer_to_answer(&self) -> bool {
        self.history.game().draw_offer() == Some(self.side_to_move().opponent())
    }

    /// The title and status on the left; the game's buttons on the right.
    fn header(&self, cx: &mut Context<Self>) -> Div {
        h_flex()
            .justify_between()
            .child(
                v_flex()
                    .gap_1()
                    .child(Label::new("Play").text_2xl())
                    .child(Label::new(status_text(self.history.game())).text_lg()),
            )
            .child(self.actions(cx))
    }

    /// Flip always; then Analyze and New game once it is over, Accept/Decline for an offer, or
    /// Draw/Resign.
    fn actions(&self, cx: &mut Context<Self>) -> Div {
        let game = self.history.game();
        let row = h_flex().gap_2().child(flip_button(cx));
        if game.is_over() {
            row.child(analyze_button(cx)).child(new_game_button(cx))
        } else if self.has_offer_to_answer() {
            row.child(accept_draw_button(cx))
                .child(decline_draw_button(cx))
        } else {
            let already_offered = game.draw_offer() == Some(self.side_to_move());
            row.child(offer_draw_button(already_offered, cx))
                .child(resign_button(cx))
        }
    }

    fn board(&self, page: WeakEntity<Self>) -> BoardGrid {
        let shown = self.history.shown();
        let board = self.shown_board();
        let draggable = if self.interaction.accepts_moves(self.can_play()) {
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
        .promotion(self.promotion_prompt(page.clone()))
        .on_right_button(right_down(page.clone()), right_up(page.clone()))
        .on_square_click(click_square(page.clone()))
        .on_piece_drop(drop_piece(page))
    }

    /// The live pieces as the premoves leave them; an earlier position as it was.
    fn shown_board(&self) -> Board {
        if self.history.is_live() {
            self.preview()
        } else {
            *self.history.shown().board()
        }
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
        if self.pending_premove {
            self.waiting_side()
        } else {
            self.side_to_move()
        }
    }

    fn promotion_prompt(&self, page: WeakEntity<Self>) -> Option<PromotionPrompt> {
        self.interaction.is_promoting().then(|| PromotionPrompt {
            side: self.promoting_side(),
            on_pick: Rc::new(pick_promotion(page.clone())),
            on_cancel: Rc::new(cancel_promotion(page)),
        })
    }

    fn navigation(&self, page: WeakEntity<Self>) -> NavigationBar {
        NavigationBar::new(
            self.history.can_go_back(),
            self.history.can_go_forward(),
            step_history(page),
        )
    }

    /// The navigation buttons, the move list, and the shown position's FEN.
    fn side_column(&self, page: WeakEntity<Self>) -> Div {
        v_flex()
            .gap_4()
            .child(self.navigation(page.clone()))
            .child(MoveList::new(
                self.history.sans().to_vec(),
                self.history.viewing(),
                select_move(page),
            ))
            .child(PositionPanel::new(*self.history.shown()))
    }

    /// The board between two player bars: the side drawn at the top above it, the other below.
    fn board_column(&self, page: WeakEntity<Self>) -> Div {
        let bottom = if self.interaction.flipped() {
            Side::Black
        } else {
            Side::White
        };
        v_flex()
            .h(relative(0.9))
            .flex_shrink_0()
            .gap_2()
            .child(self.player_bar(bottom.opponent()))
            .child(div().flex_1().min_h_0().child(self.board(page)))
            .child(self.player_bar(bottom))
    }

    /// One side's captured pieces and, in a timed game, its clock.
    fn player_bar(&self, side: Side) -> Div {
        h_flex()
            .w_full()
            .justify_between()
            .gap_4()
            .child(CapturedRow::for_side(self.history.shown().board(), side))
            .when_some(self.time_left(side), |this, left| {
                this.child(ClockFace::new(left, self.running_side() == Some(side)))
            })
    }

    fn setup_view(&self, page: WeakEntity<Self>) -> Div {
        v_flex()
            .size_full()
            .p_6()
            .gap_4()
            .child(Label::new("Play").text_2xl())
            .child(div().flex().justify_center().child(GameSetup::new(
                self.setup_choice,
                choose_time(page.clone()),
                start_game(page),
            )))
    }

    fn game_view(&self, page: WeakEntity<Self>, cx: &mut Context<Self>) -> Div {
        v_flex()
            .size_full()
            .p_6()
            .gap_4()
            .child(self.header(cx))
            .child(board_area(
                self.board_column(page.clone()),
                self.side_column(page),
            ))
    }
}

impl EventEmitter<AnalyzeGame> for PlayPage {}

impl Render for PlayPage {
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
            .child(match self.stage {
                Stage::Setup => self.setup_view(page),
                Stage::Playing => self.game_view(page, cx),
            })
    }
}

fn flip_button(cx: &mut Context<PlayPage>) -> Button {
    Button::new("flip-board")
        .ghost()
        .icon(IconName::ArrowUpDown)
        .tooltip("Flip board")
        .on_click(cx.listener(|page, _, _, cx| page.flip(cx)))
}

fn analyze_button(cx: &mut Context<PlayPage>) -> Button {
    Button::new("analyze-game")
        .outline()
        .icon(IconName::Microscope)
        .label("Analyze")
        .on_click(cx.listener(|page, _, _, cx| page.analyze(cx)))
}

fn new_game_button(cx: &mut Context<PlayPage>) -> Button {
    Button::new("new-game")
        .primary()
        .label("New game")
        .on_click(cx.listener(|page, _, _, cx| page.new_game(cx)))
}

fn offer_draw_button(already_offered: bool, cx: &mut Context<PlayPage>) -> Button {
    Button::new("offer-draw")
        .outline()
        .icon(IconName::Handshake)
        .label("Offer draw")
        .disabled(already_offered)
        .on_click(cx.listener(|page, _, _, cx| page.offer_draw(cx)))
}

fn resign_button(cx: &mut Context<PlayPage>) -> Button {
    Button::new("resign")
        .outline()
        .icon(IconName::Flag)
        .label("Resign")
        .on_click(cx.listener(|page, _, _, cx| page.resign(cx)))
}

fn accept_draw_button(cx: &mut Context<PlayPage>) -> Button {
    Button::new("accept-draw")
        .primary()
        .label("Accept draw")
        .on_click(cx.listener(|page, _, _, cx| page.accept_draw(cx)))
}

fn decline_draw_button(cx: &mut Context<PlayPage>) -> Button {
    Button::new("decline-draw")
        .outline()
        .label("Decline")
        .on_click(cx.listener(|page, _, _, cx| page.decline_draw(cx)))
}

/// Fills the space under the title: the board column on the left, the side column beside it.
fn board_area(board_column: Div, side: Div) -> Div {
    h_flex()
        .flex_1()
        .min_h_0()
        .items_start()
        .justify_center()
        .gap_6()
        .child(board_column)
        .child(side)
}

// The page outlives what it renders; a failed update means the window is closing.

fn choose_time(
    page: WeakEntity<PlayPage>,
) -> impl Fn(Option<TimeControl>, &mut Window, &mut App) + 'static {
    move |choice, _, cx| {
        let _ = page.update(cx, |page, cx| page.choose_time(choice, cx));
    }
}

fn start_game(page: WeakEntity<PlayPage>) -> impl Fn(&mut Window, &mut App) + 'static {
    move |_, cx| {
        let _ = page.update(cx, |page, cx| page.start_game(cx));
    }
}

/// Wakes every tick until the game is over or the page is gone.
fn start_ticker(cx: &mut Context<PlayPage>) -> Task<()> {
    cx.spawn(async move |page, cx| {
        loop {
            cx.background_executor().timer(TICK).await;
            let keep_going = page.update(cx, |page, cx| page.tick(cx)).unwrap_or(false);
            if !keep_going {
                break;
            }
        }
    })
}

fn step_history(page: WeakEntity<PlayPage>) -> impl Fn(NavStep, &mut Window, &mut App) + 'static {
    move |step, _, cx| {
        let _ = page.update(cx, |page, cx| page.navigate(step, cx));
    }
}

fn select_move(page: WeakEntity<PlayPage>) -> impl Fn(usize, &mut Window, &mut App) + 'static {
    move |index, _, cx| {
        let _ = page.update(cx, |page, cx| page.select_move(index, cx));
    }
}

fn click_square(page: WeakEntity<PlayPage>) -> impl Fn(Square, &mut Window, &mut App) + 'static {
    move |square, _, cx| {
        let _ = page.update(cx, |page, cx| page.click_square(square, cx));
    }
}

fn right_down(page: WeakEntity<PlayPage>) -> impl Fn(Square, &mut Window, &mut App) + 'static {
    move |square, _, cx| {
        let _ = page.update(cx, |page, cx| page.right_down(square, cx));
    }
}

fn right_up(page: WeakEntity<PlayPage>) -> impl Fn(Square, &mut Window, &mut App) + 'static {
    move |square, _, cx| {
        let _ = page.update(cx, |page, cx| page.right_up(square, cx));
    }
}

fn drop_piece(
    page: WeakEntity<PlayPage>,
) -> impl Fn(Square, Square, &mut Window, &mut App) + 'static {
    move |from, to, _, cx| {
        let _ = page.update(cx, |page, cx| page.drop_piece(from, to, cx));
    }
}

fn pick_promotion(
    page: WeakEntity<PlayPage>,
) -> impl Fn(PieceKind, &mut Window, &mut App) + 'static {
    move |kind, _, cx| {
        let _ = page.update(cx, |page, cx| page.promote(kind, cx));
    }
}

fn cancel_promotion(page: WeakEntity<PlayPage>) -> impl Fn(&mut Window, &mut App) + 'static {
    move |_, cx| {
        let _ = page.update(cx, |page, cx| page.cancel_promotion(cx));
    }
}

#[cfg(test)]
mod tests {
    use chess_domain::{Move, Side, Square, TimeControl};
    use gpui_kit::{Entity, TestAppContext, VisualTestContext};

    use super::PlayPage;
    use crate::{
        enums::history::NavStep,
        views::testing::{click, drag, lift, release},
    };

    fn sq(name: &str) -> Square {
        name.parse().expect("a square")
    }

    fn mv(from: &str, to: &str) -> Move {
        Move::new(sq(from), sq(to))
    }

    /// A Play page with a game started, with a 3 | 2 clock or none.
    fn started(cx: &mut TestAppContext, timed: bool) -> (Entity<PlayPage>, &mut VisualTestContext) {
        cx.update(gpui_kit::init);
        let (page, cx) = cx.add_window_view(|_, cx| PlayPage::new(cx));
        page.update(cx, |page, cx| {
            page.choose_time(timed.then(|| TimeControl::new(3, 2)), cx);
            page.start_game(cx);
        });
        (page, cx)
    }

    #[gpui_kit::test]
    fn a_premove_is_queued_and_plays_after_the_opponent_moves(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        page.update(cx, |page, cx| {
            page.click_square(sq("e7"), cx);
            assert_eq!(page.interaction.selected(), Some(sq("e7")));
            page.click_square(sq("e5"), cx);
            assert_eq!(page.premoves.first(Side::Black), Some(mv("e7", "e5")));
            page.click_square(sq("e2"), cx);
            page.click_square(sq("e4"), cx);
            assert_eq!(page.history.sans(), ["e4", "e5"]);
            assert!(page.premoves.is_empty());
        });
    }

    #[gpui_kit::test]
    fn a_dragged_premove_is_queued(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        page.update(cx, |page, cx| {
            page.drop_piece(sq("g8"), sq("f6"), cx);
            page.drop_piece(sq("f6"), sq("g4"), cx);
            assert_eq!(page.premoves.pairs().len(), 2);
        });
    }

    #[gpui_kit::test]
    fn a_premove_that_mates_drops_every_other_premove(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        page.update(cx, |page, cx| {
            page.drop_piece(sq("f2"), sq("f3"), cx);
            page.drop_piece(sq("e7"), sq("e5"), cx);
            page.drop_piece(sq("d8"), sq("h4"), cx);
            page.drop_piece(sq("a7"), sq("a6"), cx);
            assert_eq!(page.premoves.pairs().len(), 2, "two black premoves queued");
            page.drop_piece(sq("g2"), sq("g4"), cx);
            assert_eq!(page.history.sans(), ["f3", "e5", "g4", "Qh4#"]);
            assert!(
                page.premoves.is_empty(),
                "a6 is dropped once the game is over"
            );
        });
    }

    #[gpui_kit::test]
    fn looking_back_cancels_the_premoves(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        page.update(cx, |page, cx| {
            page.drop_piece(sq("e2"), sq("e4"), cx);
            page.drop_piece(sq("g1"), sq("f3"), cx);
            assert!(!page.premoves.is_empty());
            page.navigate(NavStep::Back, cx);
            assert!(
                page.premoves.is_empty(),
                "no red squares on an earlier position"
            );
        });
    }

    #[gpui_kit::test]
    fn real_clicks_on_the_board_move_and_premove(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        for selector in ["square-e7", "square-e5", "square-e2", "square-e4"] {
            click(cx, selector);
        }
        page.update(cx, |page, _| assert_eq!(page.history.sans(), ["e4", "e5"]));
    }

    #[gpui_kit::test]
    fn real_clicks_premove_in_a_timed_game(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, true);
        for selector in ["square-e7", "square-e5", "square-e2", "square-e4"] {
            click(cx, selector);
        }
        page.update(cx, |page, _| assert_eq!(page.history.sans(), ["e4", "e5"]));
    }

    #[gpui_kit::test]
    fn a_dragged_piece_leaves_its_square_until_it_is_dropped(cx: &mut TestAppContext) {
        let (_, cx) = started(cx, false);
        let start = lift(cx, "square-e2");
        assert!(
            cx.debug_bounds("piece-e2").is_none(),
            "only the dragged copy shows"
        );
        assert!(
            cx.debug_bounds("piece-d2").is_some(),
            "the other pieces stay"
        );
        release(cx, start);
        assert!(
            cx.debug_bounds("piece-e2").is_some(),
            "dropped back where it was"
        );
    }

    #[gpui_kit::test]
    fn real_drags_move_and_premove(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, true);
        drag(cx, "square-g8", "square-f6");
        page.update(cx, |page, _| {
            assert_eq!(
                page.premoves.first(Side::Black),
                Some(mv("g8", "f6")),
                "black premove queued by dragging"
            );
        });
        drag(cx, "square-e2", "square-e4");
        page.update(cx, |page, _| assert_eq!(page.history.sans(), ["e4", "Nf6"]));
    }
}
