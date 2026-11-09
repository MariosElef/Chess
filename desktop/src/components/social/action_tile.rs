use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, h_flex},
    prelude::FluentBuilder as _,
    *,
};

use crate::components::panel::START_GREEN;

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// A wide button atop the Friends page: its icon, bold name, a green count when waiting, an arrow.
/// The narrowest a tile goes before the row wraps, the next tile moving onto a row of its own.
const MIN_WIDTH: Pixels = px(220.);

#[derive(IntoElement)]
pub struct ActionTile {
    id: SharedString,
    icon: IconName,
    label: SharedString,
    count: usize,
    on_click: OnClick,
}

impl ActionTile {
    pub fn new(
        id: impl Into<SharedString>,
        (icon, label): (IconName, impl Into<SharedString>),
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            label: label.into(),
            count: 0,
            on_click: Rc::new(on_click),
        }
    }

    pub fn count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }
}

impl RenderOnce for ActionTile {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (ground, hover) = (theme.secondary, theme.secondary_hover);
        let on_click = Rc::clone(&self.on_click);
        let selector = self.id.to_string();
        h_flex()
            .id(self.id)
            .debug_selector(move || selector.clone())
            .flex_1()
            .min_w(MIN_WIDTH)
            .gap_3()
            .px_4()
            .h(px(52.))
            .rounded(px(6.))
            .cursor_pointer()
            .bg(ground)
            .hover(move |this| this.bg(hover))
            .child(Icon::from(self.icon).size_5().flex_shrink_0())
            .child(
                // Should the tile still be narrower than its label, the label ends in an ellipsis.
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .font_weight(FontWeight::BOLD)
                    .child(self.label),
            )
            .when(self.count > 0, |this| {
                this.child(
                    div()
                        .flex_shrink_0()
                        .px_2()
                        .rounded_full()
                        .bg(rgb(START_GREEN))
                        .text_color(white())
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .child(self.count.to_string()),
                )
            })
            .child(Icon::from(IconName::ChevronRight).size_4().flex_shrink_0())
            .on_click(move |_, window, cx| on_click(window, cx))
    }
}
