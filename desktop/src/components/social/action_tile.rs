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
            .gap_3()
            .px_4()
            .h(px(52.))
            .rounded(px(6.))
            .cursor_pointer()
            .bg(ground)
            .hover(move |this| this.bg(hover))
            .child(Icon::from(self.icon).size_5())
            .child(
                div()
                    .flex_1()
                    .font_weight(FontWeight::BOLD)
                    .child(self.label),
            )
            .when(self.count > 0, |this| {
                this.child(
                    div()
                        .px_2()
                        .rounded_full()
                        .bg(rgb(START_GREEN))
                        .text_color(white())
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .child(self.count.to_string()),
                )
            })
            .child(Icon::from(IconName::ChevronRight).size_4())
            .on_click(move |_, window, cx| on_click(window, cx))
    }
}
