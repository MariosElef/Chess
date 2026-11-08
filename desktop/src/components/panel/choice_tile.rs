use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, h_flex},
    prelude::FluentBuilder as _,
    *,
};

use super::start_button::START_GREEN;

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// One of several choices: a tile with an icon and a label, ringed in green once picked.
#[derive(IntoElement)]
pub struct ChoiceTile {
    id: SharedString,
    label: SharedString,
    icon: Option<IconName>,
    /// A character in place of an icon, such as ½ for a draw.
    glyph: Option<&'static str>,
    picked: bool,
    disabled: bool,
    height: Pixels,
    /// The icon over the label, for a narrow tile, as the end-of-game card's.
    stacked: bool,
    on_click: OnClick,
}

impl ChoiceTile {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        picked: bool,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            glyph: None,
            picked,
            disabled: false,
            height: px(42.),
            stacked: false,
            on_click: Rc::new(on_click),
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn glyph(mut self, glyph: &'static str) -> Self {
        self.glyph = Some(glyph);
        self
    }

    /// Dimmed, and a click does nothing.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The icon over the label, so a tile needs half the width.
    pub fn stacked(mut self) -> Self {
        self.stacked = true;
        self.height = px(56.);
        self
    }

    pub fn height(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }
}

impl RenderOnce for ChoiceTile {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_click = self.on_click;
        let theme = cx.theme();
        let hover = theme.input;
        let selector = self.id.to_string();
        h_flex()
            .id(self.id)
            .debug_selector(move || selector.clone())
            .w_full()
            .h(self.height)
            .flex_shrink_0()
            .justify_center()
            .gap_2()
            .px_2()
            .when(self.stacked, |this| this.flex_col().gap_1())
            .rounded(px(6.))
            .border_2()
            .bg(theme.accent)
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .when(self.picked, |this| this.border_color(rgb(START_GREEN)))
            .when(!self.picked, |this| this.border_color(transparent_black()))
            .when(!self.disabled && !self.picked, |this| {
                this.hover(move |this| this.border_color(hover))
            })
            .when(self.disabled, |this| this.opacity(0.4))
            .when(!self.disabled, |this| {
                this.cursor_pointer()
                    .on_click(move |_, window, cx| on_click(window, cx))
            })
            .when_some(self.icon, |this, icon| {
                this.child(Icon::from(icon).size_4().flex_shrink_0())
            })
            .when_some(self.glyph, |this, glyph| {
                this.child(
                    div()
                        .flex_shrink_0()
                        .text_base()
                        .font_weight(FontWeight::EXTRA_BOLD)
                        .child(glyph),
                )
            })
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(self.label),
            )
    }
}
