use gpui_kit::{
    component::{ActiveTheme as _, h_flex},
    prelude::FluentBuilder as _,
    *,
};

use super::{
    app_logo::APP_LOGO,
    window_controls::{WindowControls, draws_window_controls},
};

/// The bar across the top: the app's mark on the left, its actions on the right.
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

/// Room on the left for the Mac's close, minimise and zoom buttons; elsewhere the app draws its
/// own on the right.
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let own_controls = draws_window_controls(window);
        h_flex()
            .h(px(56.))
            .flex_shrink_0()
            .map(|this| {
                if cfg!(target_os = "macos") {
                    this.pl(WINDOW_BUTTONS)
                } else {
                    this.pl_6()
                }
            })
            .when(!own_controls, |this| this.pr_6())
            .gap_6()
            .bg(theme.sidebar)
            .text_color(theme.sidebar_foreground)
            .child(
                h_flex()
                    .id("title-bar")
                    .h_full()
                    .flex_1()
                    // Windows moves and snaps the window itself from the area it is told is the
                    // title bar; it has no move to start from a press.
                    .window_control_area(WindowControlArea::Drag)
                    .on_mouse_down(MouseButton::Left, move_window)
                    .gap_2p5()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    // The pawn is taller than wide: 290 by 400.
                    .child(img(APP_LOGO).h(px(26.)).w(px(19.)))
                    .child("Chess"),
            )
            .child(h_flex().items_center().gap_5().children(self.actions))
            .when(own_controls, |this| this.child(WindowControls))
    }
}
