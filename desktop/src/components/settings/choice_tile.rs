use std::rc::Rc;

use gpui_kit::{
    component::{ActiveTheme as _, v_flex},
    prelude::FluentBuilder as _,
    *,
};

pub type OnChoose = Rc<dyn Fn(&mut Window, &mut App)>;

/// One choice shown as a picture with its name under it; the chosen one is outlined.
#[derive(IntoElement)]
pub struct ChoiceTile {
    id: ElementId,
    /// `group-index`, for tests to find the tile.
    selector: String,
    label: &'static str,
    preview: AnyElement,
    selected: bool,
    on_choose: OnChoose,
}

impl ChoiceTile {
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

impl RenderOnce for ChoiceTile {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_choose = self.on_choose;
        let selector = self.selector;
        let theme = cx.theme();
        v_flex()
            .id(self.id)
            .debug_selector(move || selector.clone())
            .items_center()
            .gap_1p5()
            .p_2()
            .rounded(theme.radius)
            .border_2()
            .border_color(if self.selected {
                theme.ring
            } else {
                theme.transparent
            })
            .cursor_pointer()
            .hover(|this| this.bg(theme.accent))
            .when(self.selected, |this| this.bg(theme.accent))
            .on_click(move |_, window, cx| on_choose(window, cx))
            .child(self.preview)
            .child(div().text_sm().child(self.label))
    }
}
