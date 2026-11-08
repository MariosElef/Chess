use gpui_kit::{
    component::{ActiveTheme as _, v_flex},
    prelude::FluentBuilder as _,
    *,
};

/// A card of people, a shade off the page, with a small heading over them when it has one.
#[derive(IntoElement)]
pub struct PeopleList {
    heading: Option<SharedString>,
    rows: Vec<AnyElement>,
}

impl PeopleList {
    pub fn new() -> Self {
        Self {
            heading: None,
            rows: Vec::new(),
        }
    }

    pub fn heading(mut self, heading: impl Into<SharedString>) -> Self {
        self.heading = Some(heading.into());
        self
    }
}

impl ParentElement for PeopleList {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.rows.extend(elements);
    }
}

impl RenderOnce for PeopleList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .w_full()
            .gap_1()
            .p_2()
            .rounded(px(8.))
            .bg(theme.muted)
            .when_some(self.heading, |this, heading| {
                this.child(
                    div()
                        .px_3()
                        .pt_1()
                        .pb_1()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.muted_foreground)
                        .child(heading),
                )
            })
            .children(self.rows)
    }
}
