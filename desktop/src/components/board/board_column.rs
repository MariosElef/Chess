use gpui_kit::{
    component::{h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

/// The player bar's height, and the slot kept above and below every board so boards match in size.
pub const PLAYER_BAR: Pixels = px(40.);
/// The panel beside the board, at its full width and as narrow as it goes in a small window.
const SIDE_WIDTH: Pixels = px(420.);
const SIDE_MIN_WIDTH: Pixels = px(300.);
/// How much faster than the board the panel narrows.
const SIDE_SHRINK: f32 = 4.;
/// The space between the bars and the board, where what floats beside the board starts.
const BAR_GAP: Pixels = px(8.);

/// A board between two bar slots, filling the page's height.
#[derive(IntoElement)]
pub struct BoardColumn {
    board: AnyElement,
    top: Option<AnyElement>,
    bottom: Option<AnyElement>,
    /// Small buttons just outside the board's top-right corner, such as Flip board.
    tools: Vec<AnyElement>,
    /// A strip just outside the board's left edge, its full height, such as the eval bar.
    left_strip: Option<AnyElement>,
}

impl BoardColumn {
    pub fn new(board: impl IntoElement) -> Self {
        Self {
            board: board.into_any_element(),
            top: None,
            bottom: None,
            tools: Vec::new(),
            left_strip: None,
        }
    }

    pub fn left_strip(mut self, strip: impl IntoElement) -> Self {
        self.left_strip = Some(strip.into_any_element());
        self
    }

    pub fn tool(mut self, tool: impl IntoElement) -> Self {
        self.tools.push(tool.into_any_element());
        self
    }

    pub fn top(mut self, bar: impl IntoElement) -> Self {
        self.top = Some(bar.into_any_element());
        self
    }

    pub fn bottom(mut self, bar: impl IntoElement) -> Self {
        self.bottom = Some(bar.into_any_element());
        self
    }
}

impl RenderOnce for BoardColumn {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        // The tools float just outside the board's right edge, level with its top, and take no
        // room, so every page lays its board out the same with or without them. The column
        // narrows with a narrow window, the board staying square inside it.
        v_flex()
            .relative()
            .h_full()
            .min_w_0()
            .gap(BAR_GAP)
            // A strip's room is kept beside every board, whether a strip shows or not: a narrow
            // window cannot push it under the rail, and the eval bar coming and going with
            // Stockfish never moves the board.
            .ml(LEFT_STRIP_ROOM)
            .child(bar_slot(self.top))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(board_square(self.board, self.left_strip)),
            )
            .child(bar_slot(self.bottom))
            .when(!self.tools.is_empty(), |this| {
                this.child(
                    v_flex()
                        .absolute()
                        .top(PLAYER_BAR + BAR_GAP)
                        .left(relative(1.))
                        .ml_1()
                        .gap_1()
                        .children(self.tools.into_iter().map(tool_slot)),
                )
            })
    }
}

/// The board in a square as tall as its slot allows and no wider than the column, with the strip
/// just outside its left edge, exactly as tall as the board.
fn board_square(board: AnyElement, strip: Option<AnyElement>) -> Div {
    div()
        .relative()
        .h_full()
        .max_w_full()
        .aspect_square()
        .child(board)
        .when_some(strip, |this, strip| {
            this.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .right(relative(1.))
                    .mr_2()
                    .child(strip),
            )
        })
}

/// The room a left strip takes beside the board: the eval bar's width and its gap.
const LEFT_STRIP_ROOM: Pixels = px(32.);

/// The size of a tool's place beside the board: every tool centred in one, so they line up.
const TOOL_SLOT: Pixels = px(32.);

/// A tool in its square, centred, whatever kind of button it is; a tool that opens more buttons,
/// as Copy does, grows downward from it.
fn tool_slot(tool: AnyElement) -> Div {
    div()
        .min_w(TOOL_SLOT)
        .min_h(TOOL_SLOT)
        .flex()
        .items_start()
        .justify_center()
        .child(div().min_h(TOOL_SLOT).flex().items_center().child(tool))
}

fn bar_slot(bar: Option<AnyElement>) -> Div {
    div()
        .w_full()
        .h(PLAYER_BAR)
        .flex_shrink_0()
        .flex()
        .items_center()
        .when_some(bar, |this, bar| this.child(bar))
}

/// A page with a board: the board's column on the left and what goes beside it on the right.
pub fn board_page(board_column: impl IntoElement, side: impl IntoElement) -> Div {
    h_flex()
        .size_full()
        .items_start()
        .justify_center()
        .gap_10()
        .p_4()
        .child(board_column)
        .child(
            // In a narrow window the panel gives up its width well before the board does, down
            // to what its contents still need.
            div()
                .h_full()
                .flex()
                .flex_shrink(SIDE_SHRINK)
                .w(SIDE_WIDTH)
                .min_w(SIDE_MIN_WIDTH)
                .child(side),
        )
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Context, IntoElement, Render, TestAppContext, Window, div, prelude::*};

    use super::BoardColumn;

    /// A board column, with a strip on the board's left or without one.
    struct Harness {
        strip: bool,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let board = div().size_full().debug_selector(|| "board".to_owned());
            BoardColumn::new(board)
                .when(self.strip, |column| column.left_strip(div().w_4().h_full()))
        }
    }

    #[gpui_kit::test]
    fn a_strip_coming_and_going_leaves_the_board_where_it_was(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (harness, cx) = cx.add_window_view(|_, _| Harness { strip: false });
        cx.run_until_parked();
        let without = cx.debug_bounds("board").expect("the board is drawn");
        harness.update(cx, |harness, cx| {
            harness.strip = true;
            cx.notify();
        });
        cx.run_until_parked();
        let with = cx.debug_bounds("board").expect("the board is drawn");
        assert_eq!(with, without, "the eval bar takes no room from the board");
    }
}
