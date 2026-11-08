use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use crate::enums::play::PlayMode;

type OnMode = Rc<dyn Fn(PlayMode, &mut Window, &mut App)>;

/// The panel beside the board before a game, as chess.com has it: a tab for each way to play
/// across its top, and that way's choices under them.
#[derive(IntoElement)]
pub struct PlayPanel {
    mode: PlayMode,
    on_mode: OnMode,
    content: Vec<AnyElement>,
}

impl PlayPanel {
    pub fn new(
        mode: PlayMode,
        on_mode: impl Fn(PlayMode, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            mode,
            on_mode: Rc::new(on_mode),
            content: Vec::new(),
        }
    }
}

impl ParentElement for PlayPanel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.content.extend(elements);
    }
}

impl RenderOnce for PlayPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tabs: Vec<Stateful<Div>> = PlayMode::ALL
            .into_iter()
            .map(|mode| self.tab(mode, cx))
            .collect();
        v_flex()
            .w(px(420.))
            .h_full()
            .flex_shrink_0()
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .bg(cx.theme().group_box)
            .child(h_flex().w_full().children(tabs))
            .child(v_flex().px_5().py_3().gap_3().children(self.content))
    }
}

impl PlayPanel {
    /// A tab: its icon over its name; the others sit a shade darker than the panel.
    fn tab(&self, mode: PlayMode, cx: &App) -> Stateful<Div> {
        let on_mode = Rc::clone(&self.on_mode);
        let chosen = mode == self.mode;
        let theme = cx.theme();
        v_flex()
            .id(SharedString::from(format!("mode-{}", mode.label())))
            .debug_selector(move || format!("mode-{}", mode.label()))
            .flex_1()
            .items_center()
            .gap_1()
            .py_3()
            .cursor_pointer()
            .text_xs()
            .when(chosen, |this| {
                this.text_color(theme.foreground)
                    .font_weight(FontWeight::SEMIBOLD)
            })
            .when(!chosen, |this| {
                this.bg(theme.muted)
                    .text_color(theme.muted_foreground)
                    .hover(|this| this.text_color(theme.foreground))
            })
            .child(Icon::from(mode_icon(mode)).size_5())
            .child(mode.label())
            .on_click(move |_, window, cx| on_mode(mode, window, cx))
    }
}

fn mode_icon(mode: PlayMode) -> IconName {
    match mode {
        PlayMode::Online => IconName::Globe,
        PlayMode::Bots => IconName::Bot,
        PlayMode::Local => IconName::Users,
    }
}

/// The page before a game: the board between its two bars on the left, the panel on the right.
pub fn lobby(board_column: Div, panel: PlayPanel) -> Div {
    h_flex()
        .size_full()
        .items_start()
        .justify_center()
        .gap_6()
        .p_4()
        .child(board_column)
        .child(div().h_full().child(panel))
}
