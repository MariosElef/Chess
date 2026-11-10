use gpui_kit::{
    component::{ActiveTheme as _, Icon, IconName, Sizable as _, h_flex},
    prelude::FluentBuilder as _,
    *,
};

/// The width of each button, as Windows draws its own.
const BUTTON_WIDTH: Pixels = px(46.);

/// Minimise, maximise and close, on the right of the header, where the app draws the title bar
/// and the system draws no buttons: on Windows, and on Linux when the app decorates its window.
#[derive(IntoElement)]
pub struct WindowControls;

/// Whether the buttons are the app's to draw; the Mac draws its own on the left.
pub fn draws_window_controls(window: &Window) -> bool {
    if cfg!(target_os = "windows") {
        return true;
    }
    cfg!(target_os = "linux") && matches!(window.window_decorations(), Decorations::Client { .. })
}

#[derive(Clone, Copy)]
enum Control {
    Minimize,
    Maximize,
    Close,
}

impl Control {
    fn id(self) -> &'static str {
        match self {
            Self::Minimize => "window-minimize",
            Self::Maximize => "window-maximize",
            Self::Close => "window-close",
        }
    }

    fn icon(self, maximized: bool) -> IconName {
        match self {
            Self::Minimize => IconName::WindowMinimize,
            Self::Maximize if maximized => IconName::WindowRestore,
            Self::Maximize => IconName::WindowMaximize,
            Self::Close => IconName::WindowClose,
        }
    }

    /// On Windows the system handles the button itself, with its hover menu of layouts.
    fn area(self) -> WindowControlArea {
        match self {
            Self::Minimize => WindowControlArea::Min,
            Self::Maximize => WindowControlArea::Max,
            Self::Close => WindowControlArea::Close,
        }
    }

    /// Elsewhere a click does it.
    fn act(self, window: &mut Window) {
        match self {
            Self::Minimize => window.minimize_window(),
            Self::Maximize => window.zoom_window(),
            Self::Close => window.remove_window(),
        }
    }
}

impl RenderOnce for WindowControls {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let maximized = window.is_maximized();
        let supported = window.window_controls();
        h_flex()
            .h_full()
            .flex_shrink_0()
            .when(supported.minimize, |this| {
                this.child(control_button(Control::Minimize, maximized, cx))
            })
            .when(supported.maximize, |this| {
                this.child(control_button(Control::Maximize, maximized, cx))
            })
            .child(control_button(Control::Close, maximized, cx))
    }
}

fn control_button(control: Control, maximized: bool, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let (hover_bg, hover_fg) = match control {
        Control::Close => (theme.danger, theme.danger_foreground),
        _ => (theme.secondary_hover, theme.secondary_foreground),
    };
    div()
        .id(control.id())
        .h_full()
        .w(BUTTON_WIDTH)
        .flex()
        .items_center()
        .justify_center()
        .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
        .window_control_area(control.area())
        // A press here is not the start of a drag of the header around it.
        .on_mouse_down(MouseButton::Left, |_, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        .on_click(move |_, window, cx| {
            cx.stop_propagation();
            control.act(window);
        })
        .child(Icon::new(control.icon(maximized)).small())
}
