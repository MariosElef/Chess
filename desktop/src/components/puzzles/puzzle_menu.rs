use std::rc::Rc;

use gpui_kit::{
    component::{
        ActiveTheme as _,
        button::{Button, ButtonVariants},
        h_flex,
        label::Label,
        slider::SliderState,
        v_flex,
    },
    *,
};

use super::range_picker::RangePicker;
use crate::{
    enums::puzzles::RushKind,
    models::puzzles::{BestRush, RatingRange},
    services::puzzles::best_text,
};

type OnPractice = Rc<dyn Fn(&mut Window, &mut App)>;
type OnRush = Rc<dyn Fn(RushKind, &mut Window, &mut App)>;

/// The Puzzles menu: the puzzle rating, Practice at a chosen difficulty, and the three kinds
/// of Puzzle Rush with the best score of each.
#[derive(IntoElement)]
pub struct PuzzleMenu {
    rating_line: String,
    slider: Entity<SliderState>,
    range: RatingRange,
    best: BestRush,
    on_practice: OnPractice,
    on_rush: OnRush,
}

impl PuzzleMenu {
    pub fn new(
        rating_line: String,
        (slider, range): (Entity<SliderState>, RatingRange),
        best: BestRush,
    ) -> Self {
        Self {
            rating_line,
            slider,
            range,
            best,
            on_practice: Rc::new(|_, _| {}),
            on_rush: Rc::new(|_, _, _| {}),
        }
    }

    pub fn on_practice(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_practice = Rc::new(handler);
        self
    }

    pub fn on_rush(mut self, handler: impl Fn(RushKind, &mut Window, &mut App) + 'static) -> Self {
        self.on_rush = Rc::new(handler);
        self
    }
}

impl RenderOnce for PuzzleMenu {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (on_practice, on_rush, best) = (self.on_practice, self.on_rush, self.best);
        v_flex()
            .w_96()
            .gap_5()
            .p_6()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(muted(self.rating_line, cx))
            .child(
                section(
                    "Practice",
                    "One puzzle at a time, at the difficulty you choose.",
                    cx,
                )
                .child(RangePicker::new(self.slider, self.range))
                .child(
                    Button::new("start-practice")
                        .primary()
                        .w_full()
                        .label("Start practice")
                        .on_click(move |_, window, cx| on_practice(window, cx)),
                ),
            )
            .child(
                section(
                    "Puzzle Rush",
                    "As many as you can, ever harder; three mistakes end it.",
                    cx,
                )
                .children(RushKind::ALL.map(|kind| {
                    let on_rush = Rc::clone(&on_rush);
                    h_flex()
                        .justify_between()
                        .child(
                            v_flex()
                                .child(Label::new(kind.label()))
                                .child(muted(best_text(best_of(best, kind)), cx)),
                        )
                        .child(
                            Button::new(kind.key())
                                .outline()
                                .label("Start")
                                .on_click(move |_, window, cx| on_rush(kind, window, cx)),
                        )
                })),
            )
    }
}

fn best_of(best: BestRush, kind: RushKind) -> Option<i32> {
    match kind {
        RushKind::ThreeMinutes => best.three_minutes,
        RushKind::FiveMinutes => best.five_minutes,
        RushKind::Survival => best.survival,
    }
}

fn section(title: &'static str, description: &'static str, cx: &App) -> Div {
    v_flex()
        .gap_3()
        .child(Label::new(title).text_lg())
        .child(muted(description.to_owned(), cx))
}

fn muted(text: String, cx: &App) -> Div {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(text)
}
