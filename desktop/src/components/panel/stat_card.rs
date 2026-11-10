use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, h_flex, v_flex},
    *,
};

use crate::i18n::uppercase;

/// A number worth showing off: an icon and the value large, its name small and in capitals under.
#[derive(IntoElement)]
pub struct StatCard {
    icon: IconName,
    colour: Hsla,
    value: SharedString,
    caption: SharedString,
}

impl StatCard {
    pub fn new(
        (icon, colour): (IconName, Hsla),
        value: impl Into<SharedString>,
        caption: impl Into<SharedString>,
    ) -> Self {
        Self {
            icon,
            colour,
            value: value.into(),
            caption: caption.into(),
        }
    }
}

impl RenderOnce for StatCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .flex_1()
            .items_center()
            .gap_1()
            .py_3()
            .rounded(px(6.))
            .bg(theme.muted)
            .child(
                h_flex()
                    .gap_2()
                    .child(Icon::from(self.icon).size_5().text_color(self.colour))
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::EXTRA_BOLD)
                            .child(self.value),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.muted_foreground)
                    .child(uppercase(&self.caption)),
            )
    }
}
