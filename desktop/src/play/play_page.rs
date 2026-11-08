use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use chess_domain::{Clock, Move, PieceKind, Position, Side, Square, TimeControl};
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

use super::{
    annotations::Annotations,
    captured_row::CapturedRow,
    clock_face::ClockFace,
    game_setup::GameSetup,
    history::GameHistory,
    keys::{CONTEXT, FirstPosition, LivePosition, NextPosition, PreviousPosition},
    move_list::MoveList,
    position_panel::PositionPanel,
    selection::{ClickOutcome, click_outcome, targets},
    status::status_text,
    time_controls,
};

/// How often the running clock is redrawn and checked for a flag.
const TICK: Duration = Duration::from_millis(100);

/// The page shows the new-game menu until a game starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Setup,
    Playing,
}
use crate::board::{BoardGrid, PromotionPrompt};

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
    selected: Option<Square>,
    /// Black at the bottom.
    flipped: bool,
    /// A pawn waiting on the last rank while the player picks what it becomes.
    pending_promotion: Option<(Square, Square)>,
    /// Red squares and arrows drawn with the right button.
    annotations: Annotations,
    /// Where the right button went down, until it comes back up.
    right_press: Option<Square>,
    /// Keyboard focus, so the arrow keys reach this page.
    focus_handle: FocusHandle,
}

impl PlayPage {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            stage: Stage::Setup,
            setup_choice: time_controls::DEFAULT,
            history: GameHistory::new(),
            clock: None,
            turn_started: None,
            _ticker: None,
            selected: None,
            flipped: false,
            pending_promotion: None,
            annotations: Annotations::default(),
            right_press: None,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus_handle.focus(window, cx);
    }

    /// Moves can be made only on the live position of a game still going, with no picker open.
    fn can_play(&self) -> bool {
        self.history.is_live() && !self.history.game().is_over() && self.pending_promotion.is_none()
    }

    /// A left-click wipes the drawings, as on chess.com, then plays or selects.
    fn click_square(&mut self, square: Square, cx: &mut Context<Self>) {
        self.annotations.clear();
        cx.notify();
        if self.can_play() {
            let outcome = click_outcome(self.history.live(), self.selected, square);
            self.apply(outcome);
            cx.notify();
        }
    }

    /// A drop on a square the piece cannot reach puts it back.
    fn drop_piece(&mut self, from: Square, to: Square, cx: &mut Context<Self>) {
        self.annotations.clear();
        if self.can_play() {
            match click_outcome(self.history.live(), Some(from), to) {
                ClickOutcome::Select(_) => self.selected = None,
                outcome @ (ClickOutcome::Play(_) | ClickOutcome::Promote { .. }) => {
                    self.apply(outcome);
                }
            }
            cx.notify();
        }
    }

    fn apply(&mut self, outcome: ClickOutcome) {
        match outcome {
            ClickOutcome::Select(selected) => self.selected = selected,
            ClickOutcome::Play(mv) => self.play(mv),
            ClickOutcome::Promote { from, to } => {
                self.selected = Some(from);
                self.pending_promotion = Some((from, to));
            }
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

    /// Plays a move picked from the legal ones and puts the piece down; the mover's clock is
    /// charged, and the opponent's starts unless the move ended the game.
    fn play(&mut self, mv: Move) {
        let mover = self.side_to_move();
        self.charge_clock(mover, Clock::finish_turn);
        self.history
            .play(mv)
            .expect("moves come from the legal ones");
        self.turn_started = (!self.history.game().is_over()).then(Instant::now);
        self.selected = None;
        self.annotations.clear();
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
        if let Some((from, to)) = self.pending_promotion.take() {
            self.play(Move::promoting(from, to, kind));
            cx.notify();
        }
    }

    fn cancel_promotion(&mut self, cx: &mut Context<Self>) {
        self.pending_promotion = None;
        self.selected = None;
        cx.notify();
    }

    fn right_down(&mut self, square: Square) {
        self.right_press = Some(square);
    }

    /// Releasing where it was pressed marks the square; elsewhere, it draws an arrow.
    fn right_up(&mut self, square: Square, cx: &mut Context<Self>) {
        if let Some(start) = self.right_press.take() {
            self.annotations.release(start, square);
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
        self.flipped = !self.flipped;
        cx.notify();
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
        self.selected = None;
        self.pending_promotion = None;
        self.annotations.clear();
        self.right_press = None;
        cx.notify();
    }

    fn previous_position(&mut self, _: &PreviousPosition, _: &mut Window, cx: &mut Context<Self>) {
        self.navigate(GameHistory::back, cx);
    }

    fn next_position(&mut self, _: &NextPosition, _: &mut Window, cx: &mut Context<Self>) {
        self.navigate(GameHistory::forward, cx);
    }

    fn first_position(&mut self, _: &FirstPosition, _: &mut Window, cx: &mut Context<Self>) {
        self.navigate(GameHistory::jump_to_start, cx);
    }

    fn live_position(&mut self, _: &LivePosition, _: &mut Window, cx: &mut Context<Self>) {
        self.navigate(GameHistory::jump_to_live, cx);
    }

    fn select_move(&mut self, index: usize, cx: &mut Context<Self>) {
        self.history.jump_to_move(index);
        self.end_interaction(cx);
    }

    /// Moves the board through the history; the picked-up piece is put down first.
    fn navigate(&mut self, step: fn(&mut GameHistory), cx: &mut Context<Self>) {
        step(&mut self.history);
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

    /// Flip always; then New game once it is over, Accept/Decline for an offer, or Draw/Resign.
    fn actions(&self, cx: &mut Context<Self>) -> Div {
        let game = self.history.game();
        let row = h_flex().gap_2().child(flip_button(cx));
        if game.is_over() {
            row.child(new_game_button(cx))
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
        BoardGrid::new(*shown.board())
            .flipped(self.flipped)
            .selected(self.selected)
            .targets(targets(shown, self.selected))
            .last_move(self.history.shown_last_move())
            .check(check_square(shown))
            .draggable_side(self.can_play().then(|| self.side_to_move()))
            .promotion(self.promotion_prompt(page.clone()))
            .marks(self.annotations.marks().to_vec())
            .arrows(self.annotations.arrows().to_vec())
            .on_right_button(right_down(page.clone()), right_up(page.clone()))
            .on_square_click(click_square(page.clone()))
            .on_piece_drop(drop_piece(page))
    }

    fn promotion_prompt(&self, page: WeakEntity<Self>) -> Option<PromotionPrompt> {
        self.pending_promotion.map(|_| PromotionPrompt {
            side: self.side_to_move(),
            on_pick: Rc::new(pick_promotion(page.clone())),
            on_cancel: Rc::new(cancel_promotion(page)),
        })
    }

    /// ⏮ ◀ ▶ ⏭ — the ends of each direction disable themselves.
    fn navigation(&self, cx: &mut Context<Self>) -> Div {
        let back = !self.history.can_go_back();
        let forward = !self.history.can_go_forward();
        h_flex()
            .gap_2()
            .child(nav_button(
                "to-start",
                IconName::ChevronsLeft,
                "First position",
                back,
                GameHistory::jump_to_start,
                cx,
            ))
            .child(nav_button(
                "back",
                IconName::ChevronLeft,
                "Previous position",
                back,
                GameHistory::back,
                cx,
            ))
            .child(nav_button(
                "forward",
                IconName::ChevronRight,
                "Next position",
                forward,
                GameHistory::forward,
                cx,
            ))
            .child(nav_button(
                "to-live",
                IconName::ChevronsRight,
                "Current position",
                forward,
                GameHistory::jump_to_live,
                cx,
            ))
    }

    /// Captured pieces, the navigation buttons, the move list, and the shown position's FEN.
    fn side_column(&self, page: WeakEntity<Self>, cx: &mut Context<Self>) -> Div {
        v_flex()
            .gap_4()
            .child(self.navigation(cx))
            .child(MoveList::new(
                self.history.sans().to_vec(),
                self.history.viewing(),
                select_move(page),
            ))
            .child(PositionPanel::new(*self.history.shown()))
    }

    /// The board between two player bars: the side drawn at the top above it, the other below.
    fn board_column(&self, page: WeakEntity<Self>) -> Div {
        let bottom = if self.flipped {
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
                self.side_column(page, cx),
            ))
    }
}

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

/// The king's square while the side to move is in check.
fn check_square(position: &Position) -> Option<Square> {
    position
        .is_check()
        .then(|| position.board().king_of(position.side_to_move()))
        .flatten()
}

fn flip_button(cx: &mut Context<PlayPage>) -> Button {
    Button::new("flip-board")
        .ghost()
        .icon(IconName::ArrowUpDown)
        .tooltip("Flip board")
        .on_click(cx.listener(|page, _, _, cx| page.flip(cx)))
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

fn nav_button(
    id: &'static str,
    icon: IconName,
    tooltip: &'static str,
    disabled: bool,
    step: fn(&mut GameHistory),
    cx: &mut Context<PlayPage>,
) -> Button {
    Button::new(id)
        .ghost()
        .icon(icon)
        .tooltip(tooltip)
        .disabled(disabled)
        .on_click(cx.listener(move |page, _, _, cx| page.navigate(step, cx)))
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
        let _ = page.update(cx, |page, _| page.right_down(square));
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
