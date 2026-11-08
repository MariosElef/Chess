use std::time::{Duration, Instant};

use chess_domain::{Clock, Move, Side, TimeControl};
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
        history::{MoveList, NavigationBar, PositionPanel},
        play::{CapturedRow, ClockFace, GameSetup},
    },
    enums::{history::NavStep, sound::Sound},
    models::{
        board::{MoveMade, ShownPosition},
        play::AnalyzeGame,
    },
    services::{
        history::GameHistory,
        play::{ClockWarning, DEFAULT_TIME_CONTROL, status_text},
    },
    views::{
        board::BoardView,
        history::{HistoryNavigation, select_move, step_history, with_history_keys},
        sound::{play_move_sound, play_sound},
    },
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
    /// The board, with what the player is doing on it and the queued premoves.
    board: Entity<BoardView>,
    _board_moves: Subscription,
    /// Who has heard the low-time warning this game.
    clock_warning: ClockWarning,
    /// Keyboard focus, so the arrow keys reach this page.
    focus_handle: FocusHandle,
}

impl PlayPage {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let page = cx.weak_entity();
        let board = cx.new(|_| {
            BoardView::new(move |cx| page.upgrade().map(|page| page.read(cx).shown_position()))
        });
        let _board_moves = cx.subscribe(&board, |page, _, MoveMade(mv), cx| {
            page.play(*mv, cx);
            cx.notify();
        });
        Self {
            stage: Stage::Setup,
            setup_choice: DEFAULT_TIME_CONTROL,
            history: GameHistory::new(),
            clock: None,
            turn_started: None,
            _ticker: None,
            board,
            _board_moves,
            clock_warning: ClockWarning::default(),
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

    /// What the board shows: the position being looked at, taking moves only while live.
    fn shown_position(&self) -> ShownPosition {
        ShownPosition {
            position: *self.history.shown(),
            last_move: self.history.shown_last_move(),
            can_move: self.can_play(),
            engine_arrow: None,
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
        self.clock_warning = ClockWarning::default();
        self.end_interaction(cx);
        play_sound(Sound::GameStart, cx);
    }

    /// Plays a move picked from the legal ones; the mover's clock is charged, and the
    /// opponent's starts unless the move ended the game. Then any premoves that are due play.
    fn play(&mut self, mv: Move, cx: &mut Context<Self>) {
        let mover = self.side_to_move();
        self.charge_clock(mover, Clock::finish_turn);
        self.record_move(mv, cx);
        self.run_premoves(cx);
    }

    /// Adds the move to the game with its sound, and the game's end sound if it was the last.
    fn record_move(&mut self, mv: Move, cx: &App) {
        let before = *self.history.live();
        let plain = mover_sound(before.side_to_move());
        self.history
            .play(mv)
            .expect("moves come from the legal ones");
        let is_over = self.history.game().is_over();
        play_move_sound(&before, self.history.live(), mv, plain, is_over, cx);
        self.turn_started = (!is_over).then(Instant::now);
    }

    /// Plays the side to move's queued premoves while each is legal, then the other side's,
    /// and so on; the first one that is not drops the rest of that side's queue, and a game
    /// that ends drops them all.
    fn run_premoves(&mut self, cx: &mut Context<Self>) {
        loop {
            let side = self.side_to_move();
            if self.history.game().is_over() {
                self.board.update(cx, |board, _| board.drop_all_premoves());
                return;
            }
            if !self.clock_allows_premove(side) {
                self.board.update(cx, |board, _| board.drop_premoves(side));
                return;
            }
            let live = *self.history.live();
            let Some(mv) = self.board.update(cx, |board, _| board.next_premove(&live)) else {
                return;
            };
            self.play_premove(side, mv, cx);
        }
    }

    /// A premove needs more than its cost left on the clock.
    fn clock_allows_premove(&self, side: Side) -> bool {
        self.clock
            .is_none_or(|clock| clock.remaining(side, Duration::ZERO) > PREMOVE_COST)
    }

    /// A premove is played at once for a fixed cost instead of the time the turn took.
    fn play_premove(&mut self, side: Side, mv: Move, cx: &App) {
        if let Some(clock) = self.clock.as_mut() {
            clock.finish_turn(side, PREMOVE_COST);
        }
        self.record_move(mv, cx);
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
        self.warn_if_low_on_time(cx);
        if self.is_flagged() {
            self.stop_clock();
            self.history
                .flag(self.side_to_move())
                .expect("the game was still going");
            self.end_interaction(cx);
            play_sound(Sound::GameEnd, cx);
            return false;
        }
        cx.notify();
        true
    }

    /// The side whose clock is running hears the warning once it is down to its last seconds.
    fn warn_if_low_on_time(&mut self, cx: &App) {
        let (Some(side), Some(clock)) = (self.running_side(), self.clock) else {
            return;
        };
        let left = self.time_left(side).unwrap_or(Duration::ZERO);
        if self
            .clock_warning
            .is_due(side, left, clock.control().initial)
        {
            play_sound(Sound::LowTime, cx);
        }
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

    fn side_to_move(&self) -> Side {
        self.history.live().side_to_move()
    }

    fn resign(&mut self, cx: &mut Context<Self>) {
        self.stop_clock();
        self.history
            .resign(self.side_to_move())
            .expect("the button only shows while the game is on");
        self.end_interaction(cx);
        play_sound(Sound::GameEnd, cx);
    }

    fn offer_draw(&mut self, cx: &mut Context<Self>) {
        self.history
            .offer_draw(self.side_to_move())
            .expect("the button only shows while the game is on");
        play_sound(Sound::Notify, cx);
        cx.notify();
    }

    fn accept_draw(&mut self, cx: &mut Context<Self>) {
        self.stop_clock();
        self.history
            .accept_draw(self.side_to_move())
            .expect("the button only shows for an offer made to this side");
        self.end_interaction(cx);
        play_sound(Sound::GameEnd, cx);
    }

    fn decline_draw(&mut self, cx: &mut Context<Self>) {
        self.history
            .decline_draw(self.side_to_move())
            .expect("the button only shows for an offer made to this side");
        cx.notify();
    }

    fn flip(&mut self, cx: &mut Context<Self>) {
        self.board.update(cx, |board, cx| board.flip(cx));
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

    /// Puts down a picked-up piece, closes the promotion picker, wipes the drawings and drops
    /// the premoves.
    fn end_interaction(&mut self, cx: &mut Context<Self>) {
        self.board.update(cx, |board, cx| board.reset(cx));
        cx.notify();
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
    fn board_column(&self, cx: &App) -> Div {
        let bottom = if self.board.read(cx).flipped() {
            Side::Black
        } else {
            Side::White
        };
        v_flex()
            .h(relative(0.9))
            .flex_shrink_0()
            .gap_2()
            .child(self.player_bar(bottom.opponent()))
            .child(div().flex_1().min_h_0().child(self.board.clone()))
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
            .child(board_area(self.board_column(cx), self.side_column(page)))
    }
}

impl EventEmitter<AnalyzeGame> for PlayPage {}

impl HistoryNavigation for PlayPage {
    /// Moves the board through the history; the picked-up piece is put down first.
    fn navigate(&mut self, step: NavStep, cx: &mut Context<Self>) {
        self.history.step(step);
        self.end_interaction(cx);
    }

    fn select_move(&mut self, index: usize, cx: &mut Context<Self>) {
        self.history.jump_to_move(index);
        self.end_interaction(cx);
    }
}

impl Render for PlayPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = cx.entity().downgrade();
        with_history_keys(v_flex(), cx)
            .track_focus(&self.focus_handle)
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

/// On the Play board two players share one computer: White's moves and Black's sound apart.
fn mover_sound(mover: Side) -> Sound {
    match mover {
        Side::White => Sound::MoveSelf,
        Side::Black => Sound::MoveOpponent,
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

#[cfg(test)]
mod tests {
    use chess_domain::{Move, Side, Square, TimeControl};
    use gpui_kit::{Entity, TestAppContext, VisualTestContext};

    use super::PlayPage;
    use crate::{
        enums::{history::NavStep, sound::Sound},
        testing::{click, drag, lift, release},
        views::history::HistoryNavigation as _,
        views::sound::{played_sounds, record_sounds},
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

    fn sans(page: &Entity<PlayPage>, cx: &mut VisualTestContext) -> Vec<String> {
        page.update(cx, |page, _| page.history.sans().to_vec())
    }

    fn premoves(page: &Entity<PlayPage>, cx: &mut VisualTestContext) -> Vec<(Square, Square)> {
        page.update(cx, |page, cx| page.board.read(cx).premoves().pairs())
    }

    #[gpui_kit::test]
    fn a_premove_is_queued_and_plays_after_the_opponent_moves(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        click(cx, "square-e7");
        let selected = page.update(cx, |page, cx| page.board.read(cx).selected());
        assert_eq!(selected, Some(sq("e7")));
        click(cx, "square-e5");
        assert_eq!(premoves(&page, cx), [(sq("e7"), sq("e5"))]);
        click(cx, "square-e2");
        click(cx, "square-e4");
        assert_eq!(sans(&page, cx), ["e4", "e5"]);
        assert!(premoves(&page, cx).is_empty());
    }

    #[gpui_kit::test]
    fn premoves_chain_through_the_preview(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        drag(cx, "square-g8", "square-f6");
        drag(cx, "square-f6", "square-g4");
        assert_eq!(premoves(&page, cx).len(), 2);
    }

    #[gpui_kit::test]
    fn a_premove_that_mates_drops_every_other_premove(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        drag(cx, "square-f2", "square-f3");
        drag(cx, "square-e7", "square-e5");
        drag(cx, "square-d8", "square-h4");
        drag(cx, "square-a7", "square-a6");
        assert_eq!(premoves(&page, cx).len(), 2, "two black premoves queued");
        drag(cx, "square-g2", "square-g4");
        assert_eq!(sans(&page, cx), ["f3", "e5", "g4", "Qh4#"]);
        assert!(
            premoves(&page, cx).is_empty(),
            "a6 is dropped once the game is over"
        );
    }

    #[gpui_kit::test]
    fn looking_back_cancels_the_premoves(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, false);
        drag(cx, "square-e2", "square-e4");
        drag(cx, "square-g1", "square-f3");
        assert!(!premoves(&page, cx).is_empty());
        page.update(cx, |page, cx| page.navigate(NavStep::Back, cx));
        assert!(
            premoves(&page, cx).is_empty(),
            "no red squares on an earlier position"
        );
    }

    #[gpui_kit::test]
    fn a_game_sounds_from_its_start_to_a_premoved_mate(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            record_sounds(cx);
        });
        let (page, cx) = cx.add_window_view(|_, cx| PlayPage::new(cx));
        page.update(cx, |page, cx| {
            page.choose_time(None, cx);
            page.start_game(cx);
        });
        drag(cx, "square-e2", "square-e5");
        drag(cx, "square-f2", "square-f3");
        drag(cx, "square-e7", "square-e5");
        drag(cx, "square-d8", "square-h4");
        drag(cx, "square-g2", "square-g4");
        let played = cx.update(|_, cx| played_sounds(cx));
        assert_eq!(
            played,
            [
                Sound::GameStart,
                Sound::Illegal,
                Sound::MoveSelf,
                Sound::MoveOpponent,
                Sound::Premove,
                Sound::MoveSelf,
                Sound::Check,
                Sound::GameEnd,
            ]
        );
    }

    #[gpui_kit::test]
    fn real_clicks_premove_in_a_timed_game(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, true);
        for selector in ["square-e7", "square-e5", "square-e2", "square-e4"] {
            click(cx, selector);
        }
        assert_eq!(sans(&page, cx), ["e4", "e5"]);
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
    fn real_drags_move_and_premove_in_a_timed_game(cx: &mut TestAppContext) {
        let (page, cx) = started(cx, true);
        drag(cx, "square-g8", "square-f6");
        let first = page.update(cx, |page, cx| {
            page.board.read(cx).premoves().first(Side::Black)
        });
        assert_eq!(
            first,
            Some(mv("g8", "f6")),
            "black premove queued by dragging"
        );
        drag(cx, "square-e2", "square-e4");
        assert_eq!(sans(&page, cx), ["e4", "Nf6"]);
    }
}
