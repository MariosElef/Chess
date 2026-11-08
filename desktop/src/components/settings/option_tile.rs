use std::rc::Rc;

use gpui_kit::{
    component::{ActiveTheme as _, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use crate::components::panel::START_GREEN;

pub type OnChoose = Rc<dyn Fn(&mut Window, &mut App)>;

/// One choice shown as a picture with its name under it; the chosen one is ringed in green.
#[derive(IntoElement)]
pub struct OptionTile {
    id: ElementId,
    /// `group-index`, for tests to find the tile.
    selector: String,
    label: &'static str,
    preview: AnyElement,
    selected: bool,
    on_choose: OnChoose,
}

impl OptionTile {
    /// `group` and `index` name the tile, for its id and the test selector `group-index`.
    pub fn new(
        group: &'static str,
        index: usize,
        label: &'static str,
        preview: impl IntoElement,
        on_choose: OnChoose,
    ) -> Self {
        Self {
            id: (group, index).into(),
            selector: format!("{group}-{index}"),
            label,
            preview: preview.into_any_element(),
            selected: false,
            on_choose,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl RenderOnce for OptionTile {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_choose = self.on_choose;
        let selector = self.selector;
        let theme = cx.theme();
        v_flex()
            .id(self.id)
            .debug_selector(move || selector.clone())
            .items_center()
            .gap_2()
            .p_3()
            .rounded(px(8.))
            .border_2()
            .border_color(if self.selected {
                rgb(START_GREEN).into()
            } else {
                theme.transparent
            })
            .bg(theme.accent)
            .cursor_pointer()
            .when(!self.selected, |this| {
                this.hover(|this| this.bg(theme.input))
            })
            .on_click(move |_, window, cx| on_choose(window, cx))
            .child(self.preview)
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.label),
            )
    }
}
