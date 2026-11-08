use gpui_kit::{
    component::{
        ActiveTheme as _, Sizable as _,
        input::{Input, InputState},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

/// A labelled text field with the server's complaint about it underneath.
#[derive(IntoElement)]
pub struct FormField {
    label: &'static str,
    state: Entity<InputState>,
    problem: Option<String>,
    mask_toggle: bool,
}

impl FormField {
    pub fn new(label: &'static str, state: Entity<InputState>, problem: Option<String>) -> Self {
        Self {
            label,
            state,
            problem,
            mask_toggle: false,
        }
    }

    /// A password: an eye button shows what was typed.
    pub fn password(mut self) -> Self {
        self.mask_toggle = true;
        self
    }
}

impl RenderOnce for FormField {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let input = Input::new(&self.state).large();
        v_flex()
            .gap_1p5()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(cx.theme().muted_foreground)
                    .child(self.label),
            )
            .child(if self.mask_toggle {
                input.mask_toggle()
            } else {
                input
            })
            .when_some(self.problem, |this, problem| {
                this.child(div().text_xs().text_color(cx.theme().danger).child(problem))
            })
    }
}
