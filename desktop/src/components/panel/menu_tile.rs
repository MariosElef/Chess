use std::rc::Rc;

use gpui_kit::{
    component::{ActiveTheme as _, h_flex},
    prelude::FluentBuilder as _,
    *,
};

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// A tall tile in a panel's menu, as chess.com's: a picture and a bold name in the middle.
#[derive(IntoElement)]
pub struct MenuTile {
    id: SharedString,
    picture: Option<&'static str>,
    label: SharedString,
    on_click: OnClick,
}

impl MenuTile {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            picture: None,
            label: label.into(),
            on_click: Rc::new(on_click),
        }
    }

    pub fn picture(mut self, picture: &'static str) -> Self {
        self.picture = Some(picture);
        self
    }
}

impl RenderOnce for MenuTile {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_click = self.on_click;
        let theme = cx.theme();
        let hover = theme.input;
        let selector = self.id.to_string();
        h_flex()
            .id(self.id)
            .debug_selector(move || selector.clone())
            .relative()
            .w_full()
            .h(px(52.))
            .flex_shrink_0()
            .justify_center()
            .gap_2p5()
            .rounded(px(8.))
            .border_2()
            .border_color(transparent_black())
            .bg(theme.accent)
            .text_lg()
            .font_weight(FontWeight::BOLD)
            .cursor_pointer()
            .hover(move |this| this.border_color(hover))
            .when_some(self.picture, |this, path| this.child(img(path).size_6()))
            .child(self.label)
            .on_click(move |_, window, cx| on_click(window, cx))
    }
}
