use gpui_kit::{
    component::{ActiveTheme as _, h_flex},
    *,
};

/// The strip across the top of a panel: its picture and name, on a band a shade darker.
#[derive(IntoElement)]
pub struct PanelHeader {
    picture: &'static str,
    title: SharedString,
}

impl PanelHeader {
    pub fn new(picture: &'static str, title: impl Into<SharedString>) -> Self {
        Self {
            picture,
            title: title.into(),
        }
    }
}

impl RenderOnce for PanelHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .w_full()
            .justify_center()
            .gap_2()
            .py_3()
            .bg(cx.theme().muted)
            .text_lg()
            .font_weight(FontWeight::BOLD)
            .child(img(self.picture).size_6())
            .child(self.title)
    }
}
