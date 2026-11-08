use std::rc::Rc;

use gpui_kit::{
    component::{
        ActiveTheme as _,
        button::Button,
        h_flex,
        input::{Textarea, TextareaState},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

type OnLoad = Rc<dyn Fn(&mut Window, &mut App)>;

/// A field to paste a FEN or a PGN into, a Load button, and what was wrong with the last one.
#[derive(IntoElement)]
pub struct FenInput {
    state: Entity<TextareaState>,
    error: Option<String>,
    on_load: OnLoad,
}

impl FenInput {
    pub fn new(
        state: Entity<TextareaState>,
        error: Option<String>,
        on_load: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            state,
            error,
            on_load: Rc::new(on_load),
        }
    }
}

impl RenderOnce for FenInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_load = Rc::clone(&self.on_load);
        v_flex()
            .gap_1()
            .child(
                h_flex()
                    .gap_2()
                    .items_start()
                    .child(
                        div()
                            .flex_1()
                            .debug_selector(|| "fen-field".to_owned())
                            .child(Textarea::new(&self.state)),
                    )
                    .child(
                        Button::new("load-fen")
                            .outline()
                            .label("Load")
                            .on_click(move |_, window, cx| on_load(window, cx)),
                    ),
            )
            .when_some(self.error, |this, error| {
                this.child(div().text_xs().text_color(cx.theme().danger).child(error))
            })
    }
}
