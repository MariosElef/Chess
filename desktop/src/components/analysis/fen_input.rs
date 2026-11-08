use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, h_flex,
        input::{Textarea, TextareaState},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use crate::components::panel::ChoiceTile;

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
        let theme = cx.theme();
        v_flex()
            .w_full()
            .gap_2()
            .p_3()
            .rounded(px(8.))
            .bg(theme.muted)
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.muted_foreground)
                            .child("LOAD A GAME"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("FEN or PGN"),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .debug_selector(|| "fen-field".to_owned())
                    .child(Textarea::new(&self.state)),
            )
            .child(
                ChoiceTile::new("load-fen", "Load", false, move |window, cx| {
                    on_load(window, cx);
                })
                .icon(IconName::Upload)
                .height(px(36.)),
            )
            .when_some(self.error, |this, error| {
                this.child(div().text_xs().text_color(cx.theme().danger).child(error))
            })
    }
}
