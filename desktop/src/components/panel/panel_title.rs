use std::rc::Rc;

use super::on_back::OnBack;
use gpui_kit::{
    assets::IconName,
    component::{
        Sizable as _,
        button::{Button, ButtonVariants},
        h_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

/// A panel's title, large, with its picture before it, and an arrow back at the left when
/// the panel opened from another.
#[derive(IntoElement)]
pub struct PanelTitle {
    picture: &'static str,
    title: SharedString,
    on_back: Option<OnBack>,
}

impl PanelTitle {
    pub fn new(picture: &'static str, title: impl Into<SharedString>) -> Self {
        Self {
            picture,
            title: title.into(),
            on_back: None,
        }
    }

    pub fn on_back(mut self, on_back: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_back = Some(Rc::new(on_back));
        self
    }
}

impl RenderOnce for PanelTitle {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        h_flex()
            .relative()
            .w_full()
            .justify_center()
            .gap_3()
            .py_2()
            .when_some(self.on_back, |this, on_back| {
                this.child(
                    div().absolute().left_0().child(
                        Button::new("panel-back")
                            .ghost()
                            .large()
                            .icon(IconName::ArrowLeft)
                            .on_click(move |_, window, cx| on_back(window, cx)),
                    ),
                )
            })
            .child(img(self.picture).size_9())
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::EXTRA_BOLD)
                    .child(self.title),
            )
    }
}
