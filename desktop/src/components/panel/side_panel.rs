use gpui_kit::{
    component::{ActiveTheme as _, scroll::ScrollableElement as _, v_flex},
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
        // What does not fit a short window scrolls, with a scrollbar, rather than being cut off.
        let content = v_flex()
            .w_full()
            .flex_1()
            .min_h_0()
            .px_5()
            .py_3()
            .gap_3()
            .children(self.content)
            .overflow_y_scrollbar();
        v_flex()
            .size_full()
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .bg(cx.theme().group_box)
            .when_some(self.header, |this, header| this.child(header))
            .child(content)
    }
}
