use gpui_kit::{
    component::{
        ActiveTheme as _, h_flex,
        label::Label,
        slider::{Slider, SliderState},
        v_flex,
    },
    *,
};

use crate::models::puzzles::RatingRange;

/// The puzzle difficulty, as a rating range with a handle at each end.
#[derive(IntoElement)]
pub struct RangePicker {
    state: Entity<SliderState>,
    range: RatingRange,
}

impl RangePicker {
    pub fn new(state: Entity<SliderState>, range: RatingRange) -> Self {
        Self { state, range }
    }
}

impl RenderOnce for RangePicker {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .p_4()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .justify_between()
                    .child(Label::new("Difficulty").text_sm())
                    .child(
                        Label::new(format!("{} – {}", self.range.min, self.range.max))
                            .text_sm()
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            .child(Slider::new(&self.state))
    }
}
