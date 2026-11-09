use std::rc::Rc;

use gpui_kit::{prelude::FluentBuilder as _, *};

/// chess.com's green, the colour of the button that starts things.
pub const START_GREEN: u32 = 0x0081_b64c;
const START_GREEN_HOVER: u32 = 0x0095_c75a;
const START_GREEN_EDGE: u32 = 0x0045_7536;

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// The big green button that starts a game or a puzzle; dimmed and deaf to clicks while it waits.
#[derive(IntoElement)]
pub struct StartButton {
    id: SharedString,
    label: SharedString,
    picture: Option<&'static str>,
    busy: bool,
    disabled: bool,
    small: bool,
    on_click: OnClick,
}

impl StartButton {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            picture: None,
            busy: false,
            disabled: false,
            small: false,
            on_click: Rc::new(on_click),
        }
    }

    /// A picture before the label, such as Game Review's star.
    pub fn picture(mut self, picture: &'static str) -> Self {
        self.picture = Some(picture);
        self
    }

    /// Dimmed and taking no clicks, while what it started is on its way.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// As wide as its label and short, for a toast's answer, with the same green edge.
    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    /// Dimmed and taking no clicks, until what it needs is filled in.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for StartButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let on_click = self.on_click;
        let inert = self.busy || self.disabled;
        let selector = self.id.to_string();
        div()
            .id(self.id)
            .debug_selector(move || selector.clone())
            .when(self.small, |this| {
                this.h(px(32.))
                    .px_4()
                    .rounded(px(6.))
                    .border_b_2()
                    .text_sm()
            })
            .when(!self.small, |this| {
                this.w_full()
                    .h(px(48.))
                    .rounded(px(8.))
                    .border_b_4()
                    .text_lg()
            })
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .gap_2p5()
            .bg(rgb(START_GREEN))
            .border_color(rgb(START_GREEN_EDGE))
            .text_color(white())
            .font_weight(FontWeight::EXTRA_BOLD)
            .when(inert, |this| this.opacity(0.7))
            .when(!inert, |this| {
                this.cursor_pointer()
                    .hover(|this| this.bg(rgb(START_GREEN_HOVER)))
            })
            .when_some(self.picture, |this, path| this.child(img(path).size_6()))
            .child(self.label)
            .when(!inert, |this| {
                this.on_click(move |_, window, cx| on_click(window, cx))
            })
    }
}
