use chess_domain::{Move, Square};
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
    history::GameHistory,
    keys::{CONTEXT, FirstPosition, LivePosition, NextPosition, PreviousPosition},
    move_list::MoveList,
    position_panel::PositionPanel,
    selection::{ClickOutcome, click_outcome, targets},
    status::{is_over, status_text},
};
use crate::board::BoardGrid;

/// The game being played, the position on the board, and which piece is picked up.
pub struct PlayPage {
    history: GameHistory,
    selected: Option<Square>,
    /// Keyboard focus, so the arrow keys reach this page.
    focus_handle: FocusHandle,
}

impl PlayPage {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            history: GameHistory::new(),
            selected: None,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus_handle.focus(window, cx);
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
        self.selected = None;
        cx.notify();
    }

    /// Only the live position can be played from; an earlier one is just for looking.
    fn click_square(&mut self, square: Square, cx: &mut Context<Self>) {
        if !self.history.is_live() {
            return;
        }
        match click_outcome(self.history.live(), self.selected, square) {
            ClickOutcome::Select(selected) => self.selected = selected,
            ClickOutcome::Play(mv) => self.play(mv),
        }
        cx.notify();
    }

    /// Plays a move picked from the legal ones and puts the piece down.
    fn play(&mut self, mv: Move) {
        self.history
            .play(mv)
            .expect("click_outcome only picks legal moves");
        self.selected = None;
    }

    fn new_game(&mut self, cx: &mut Context<Self>) {
        self.history = GameHistory::new();
        self.selected = None;
        cx.notify();
    }

    /// Moves the board through the history; the picked-up piece is put down first.
    fn navigate(&mut self, step: fn(&mut GameHistory), cx: &mut Context<Self>) {
        step(&mut self.history);
        self.selected = None;
        cx.notify();
    }

    /// The title and status on the left; a New game button on the right once the game is over.
    fn header(&self, cx: &mut Context<Self>) -> Div {
        let live = self.history.live();
        h_flex()
            .justify_between()
            .child(
                v_flex()
                    .gap_1()
                    .child(Label::new("Play").text_2xl())
                    .child(Label::new(status_text(live)).text_lg()),
            )
            .when(is_over(live), |this| this.child(new_game_button(cx)))
    }

    fn board(&self, page: WeakEntity<Self>) -> BoardGrid {
        let shown = self.history.shown();
        BoardGrid::new(*shown.board())
            .selected(self.selected)
            .targets(targets(shown, self.selected))
            .on_square_click(click_square(page))
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

    /// The navigation buttons, the move list, and the FEN panel of the position being shown.
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
            .p_6()
            .gap_4()
            .child(self.header(cx))
            .child(board_area(
                self.board(page.clone()),
                self.side_column(page, cx),
            ))
    }
}

fn new_game_button(cx: &mut Context<PlayPage>) -> Button {
    Button::new("new-game")
        .primary()
        .label("New game")
        .on_click(cx.listener(|page, _, _, cx| page.new_game(cx)))
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

/// Fills the space under the title: the board on the left, square, with the side column beside it.
fn board_area(board: BoardGrid, side: Div) -> Div {
    h_flex()
        .flex_1()
        .min_h_0()
        .items_start()
        .justify_center()
        .gap_6()
        .child(board_frame(board))
        .child(side)
}

/// Keeps the board a little shorter than the space it has.
fn board_frame(board: BoardGrid) -> Div {
    div().h(relative(0.9)).flex_shrink_0().child(board)
}

// The page outlives what it renders; a failed update means the window is closing.

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
