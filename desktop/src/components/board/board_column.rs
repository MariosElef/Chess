use gpui_kit::{
    component::{h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

/// The player bar's height, and the slot kept above and below every board so boards match in size.
pub const PLAYER_BAR: Pixels = px(40.);
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
        // The tools float just outside the board's right edge, level with its top, and the strip
        // outside its left edge: they take no room, so every page lays its board out the same
        // with or without them.
        v_flex()
            .relative()
            .h_full()
            .flex_shrink_0()
            .gap(BAR_GAP)
            .child(bar_slot(self.top))
            .child(div().flex_1().min_h_0().child(self.board))
            .child(bar_slot(self.bottom))
            .when_some(self.left_strip, |this, strip| {
                this.child(
                    div()
                        .absolute()
                        .top(PLAYER_BAR + BAR_GAP)
                        .bottom(PLAYER_BAR + BAR_GAP)
                        .right(relative(1.))
                        .mr_2()
                        .child(strip),
                )
            })
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

/// The size of a tool's place beside the board: every tool centred in one, so they line up.
const TOOL_SLOT: Pixels = px(32.);

/// A tool in its square, centred, whatever kind of button it is.
fn tool_slot(tool: AnyElement) -> Div {
    div()
        .size(TOOL_SLOT)
        .flex()
        .items_center()
        .justify_center()
        .child(tool)
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
        .child(div().h_full().child(side))
}
