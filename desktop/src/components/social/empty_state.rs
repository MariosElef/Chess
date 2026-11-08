use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, v_flex},
    *,
};

/// What a list says with nothing in it: a large icon, a line in bold, and a line under it.
#[derive(IntoElement)]
pub struct EmptyState {
    icon: IconName,
    title: SharedString,
    text: SharedString,
}

impl EmptyState {
    pub fn new(
        icon: IconName,
        title: impl Into<SharedString>,
        text: impl Into<SharedString>,
    ) -> Self {
        Self {
            icon,
            title: title.into(),
            text: text.into(),
        }
    }
}

impl RenderOnce for EmptyState {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        v_flex()
            .w_full()
            .items_center()
            .gap_2()
            .py_10()
            .child(Icon::from(self.icon).size_10().text_color(muted))
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.title),
            )
            .child(div().text_sm().text_color(muted).child(self.text))
    }
}
