use std::rc::Rc;

use gpui_kit::{assets::IconName, component::Icon, prelude::FluentBuilder as _, *};

/// chess.com's green, the colour of the button that starts things.
pub const START_GREEN: u32 = 0x0081_b64c;
const START_GREEN_HOVER: u32 = 0x0095_c75a;
const START_GREEN_EDGE: u32 = 0x0045_7536;
/// The white of a plain button, as New group, with its hover and its grey edge.
const PLAIN_WHITE: u32 = 0x00ec_ecec;
const PLAIN_WHITE_HOVER: u32 = 0x00ff_ffff;
const PLAIN_WHITE_EDGE: u32 = 0x00a8_a8a8;
const PLAIN_TEXT: u32 = 0x0020_2020;
/// The red of a button that ends something, with its hover and its darker edge.
const STOP_RED: u32 = 0x00d6_4b40;
const STOP_RED_HOVER: u32 = 0x00e2_5f54;
const STOP_RED_EDGE: u32 = 0x0089_2a22;
const WHITE: u32 = 0x00ff_ffff;

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// The app's 3D button, green, red or white in three sizes; dimmed and deaf to clicks while busy.
#[derive(IntoElement)]
pub struct StartButton {
    id: SharedString,
    label: SharedString,
    picture: Option<&'static str>,
    icon: Option<IconName>,
    busy: bool,
    disabled: bool,
    size: Size,
    /// Green to start something, red to end it, white for anything else.
    tone: Tone,
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
            icon: None,
            busy: false,
            disabled: false,
            size: Size::Full,
            tone: Tone::Start,
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

    /// An icon before the label, such as Sign out's arrow.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Red instead of green, for a button that ends something.
    pub fn danger(mut self) -> Self {
        self.tone = Tone::Stop;
        self
    }

    /// White instead of green, for a button that neither starts nor ends anything.
    pub fn plain(mut self) -> Self {
        self.tone = Tone::Plain;
        self
    }

    /// As wide as its label and short.
    pub fn small(mut self) -> Self {
        self.size = Size::Small;
        self
    }

    /// As wide as its label, taller than small and with larger text.
    pub fn medium(mut self) -> Self {
        self.size = Size::Medium;
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
        let (face, hover, edge, text) = colours(self.tone);
        div()
            .id(self.id)
            .debug_selector(move || selector.clone())
            .map(|this| match self.size {
                Size::Small => this
                    .h(px(32.))
                    .px_4()
                    .rounded(px(6.))
                    .border_b_2()
                    .text_sm(),
                Size::Medium => this
                    .h(px(42.))
                    .px_5()
                    .rounded(px(8.))
                    .border_b_4()
                    .text_base(),
                Size::Full => this
                    .w_full()
                    .h(px(48.))
                    .rounded(px(8.))
                    .border_b_4()
                    .text_lg(),
            })
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .gap_2p5()
            .bg(rgb(face))
            .border_color(rgb(edge))
            .text_color(rgb(text))
            .font_weight(FontWeight::EXTRA_BOLD)
            .when(inert, |this| this.opacity(0.7))
            .when(!inert, |this| {
                this.cursor_pointer().hover(move |this| this.bg(rgb(hover)))
            })
            .when_some(self.picture, |this, path| this.child(img(path).size_6()))
            .when_some(self.icon, |this, icon| this.child(Icon::new(icon).size_4()))
            .child(self.label)
            .when(!inert, |this| {
                this.on_click(move |_, window, cx| on_click(window, cx))
            })
    }
}

/// How big a 3D button is: short, a little taller, or the full width of its panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Size {
    Small,
    Medium,
    Full,
}

/// What a 3D button does, which sets its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tone {
    Start,
    Stop,
    Plain,
}

/// The button's colour, its hover, its darker bottom edge and its label's colour.
fn colours(tone: Tone) -> (u32, u32, u32, u32) {
    match tone {
        Tone::Start => (START_GREEN, START_GREEN_HOVER, START_GREEN_EDGE, WHITE),
        Tone::Stop => (STOP_RED, STOP_RED_HOVER, STOP_RED_EDGE, WHITE),
        Tone::Plain => (PLAIN_WHITE, PLAIN_WHITE_HOVER, PLAIN_WHITE_EDGE, PLAIN_TEXT),
    }
}
