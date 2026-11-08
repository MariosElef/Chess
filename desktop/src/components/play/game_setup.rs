use std::rc::Rc;

use chess_domain::{Category, TimeControl};
use gpui_kit::{
    component::{
        ActiveTheme as _, Selectable as _, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex,
        label::Label,
        v_flex,
    },
    *,
};

use crate::services::play::presets_in;

type OnChoose = Rc<dyn Fn(Option<TimeControl>, &mut Window, &mut App)>;
type OnStart = Rc<dyn Fn(&mut Window, &mut App)>;

/// The new-game menu: a time control by speed, or none, then Start.
#[derive(IntoElement)]
pub struct GameSetup {
    /// `None` is a game without a clock.
    choice: Option<TimeControl>,
    on_choose: OnChoose,
    on_start: OnStart,
}

impl GameSetup {
    pub fn new(
        choice: Option<TimeControl>,
        on_choose: impl Fn(Option<TimeControl>, &mut Window, &mut App) + 'static,
        on_start: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            choice,
            on_choose: Rc::new(on_choose),
            on_start: Rc::new(on_start),
        }
    }
}

impl RenderOnce for GameSetup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .w_96()
            .gap_5()
            .p_6()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(mode_heading(cx))
            .children(Category::ALL.map(|category| self.category_section(category)))
            .child(self.no_clock_button())
            .child(self.start_button())
    }
}

impl GameSetup {
    fn category_section(&self, category: Category) -> Div {
        v_flex()
            .gap_2()
            .child(Label::new(category.name()).text_sm())
            .child(
                h_flex().gap_2().children(
                    presets_in(category)
                        .into_iter()
                        .map(|control| self.preset_button(control)),
                ),
            )
    }

    fn preset_button(&self, control: TimeControl) -> Button {
        self.choice_button(
            SharedString::from(format!("time-{}", control.label())),
            control.label(),
            Some(control),
        )
        .flex_1()
    }

    fn no_clock_button(&self) -> Button {
        self.choice_button("time-none".into(), "No clock".to_owned(), None)
            .w_full()
    }

    /// Outlined until picked, filled once it is the choice.
    fn choice_button(&self, id: SharedString, label: String, value: Option<TimeControl>) -> Button {
        let on_choose = Rc::clone(&self.on_choose);
        Button::new(id)
            .outline()
            .label(label)
            .selected(self.choice == value)
            .on_click(move |_, window, cx| on_choose(value, window, cx))
    }

    fn start_button(&self) -> Button {
        let on_start = Rc::clone(&self.on_start);
        Button::new("start-game")
            .primary()
            .large()
            .w_full()
            .label("Start game")
            .on_click(move |_, window, cx| on_start(window, cx))
    }
}

fn mode_heading(cx: &App) -> Div {
    v_flex()
        .gap_1()
        .child(Label::new("New game").text_xl())
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("Local · two players on this computer"),
        )
}
