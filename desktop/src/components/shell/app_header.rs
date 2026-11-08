use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, Sizable as _, h_flex},
    *,
};

/// The bar across the top: the app's mark on the left, its actions on the right. It shares the
/// rail's colour, the two making one L around the page. It is the window's title bar too: the
/// Mac's buttons sit at its left, and dragging it moves the window.
#[derive(IntoElement)]
pub struct AppHeader {
    actions: Vec<AnyElement>,
}

impl AppHeader {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }
}

/// Room on the left for the Mac's close, minimise and zoom buttons.
const WINDOW_BUTTONS: Pixels = px(88.);

/// Where the Mac's buttons sit: centred in the 56px header.
pub const WINDOW_BUTTONS_AT: Point<Pixels> = point(px(20.), px(21.));

/// Dragging the bar moves the window; a double click zooms it, as a title bar does.
fn move_window(event: &MouseDownEvent, window: &mut Window, _: &mut App) {
    if event.click_count == 2 {
        window.titlebar_double_click();
    } else {
        window.start_window_move();
    }
}

impl RenderOnce for AppHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        h_flex()
            .h(px(56.))
            .flex_shrink_0()
            .pl(WINDOW_BUTTONS)
            .pr_6()
            .gap_6()
            .bg(theme.sidebar)
            .text_color(theme.sidebar_foreground)
            .child(
                h_flex()
                    .id("title-bar")
                    .h_full()
                    .flex_1()
                    .on_mouse_down(MouseButton::Left, move_window)
                    .gap_2p5()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(Icon::from(IconName::ChessKnight).with_size(px(26.)))
                    .child("Chess"),
            )
            .child(h_flex().items_center().gap_5().children(self.actions))
    }
}
