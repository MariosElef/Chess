use gpui_kit::{
    component::{ActiveTheme as _, v_flex},
    prelude::FluentBuilder as _,
    *,
};

/// The panel beside the board on every page: an optional strip across its top, then its content.
#[derive(IntoElement, Default)]
pub struct SidePanel {
    header: Option<AnyElement>,
    content: Vec<AnyElement>,
}

impl SidePanel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.header = Some(header.into_any_element());
        self
    }
}

impl ParentElement for SidePanel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.content.extend(elements);
    }
}

impl RenderOnce for SidePanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let content = v_flex()
            .w_full()
            .flex_1()
            .min_h_0()
            .px_5()
            .py_3()
            .gap_3()
            .children(self.content);
        v_flex()
            .w(px(420.))
            .h_full()
            .flex_shrink_0()
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .bg(cx.theme().group_box)
            .when_some(self.header, |this, header| this.child(header))
            .child(content)
    }
}
