use std::rc::Rc;

use chess_domain::{Category, TimeControl};
use gpui_kit::{
    component::{
        ActiveTheme as _, Disableable as _, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex,
        label::Label,
        switch::Switch,
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use crate::services::play::presets_in;

type OnChoose = Rc<dyn Fn(TimeControl, &mut Window, &mut App)>;
type OnRated = Rc<dyn Fn(bool, &mut Window, &mut App)>;
type OnPress = Rc<dyn Fn(&mut Window, &mut App)>;

/// What a challenge asks for: a time control by speed and rated or not, then Send.
#[derive(IntoElement)]
pub struct ChallengePicker {
    choice: TimeControl,
    rated: bool,
    /// Signed in with an account; a guest's games are never rated.
    can_rate: bool,
    on_choose: OnChoose,
    on_rated: OnRated,
    on_send: OnPress,
    on_close: OnPress,
}

impl ChallengePicker {
    pub fn new(
        (choice, rated, can_rate): (TimeControl, bool, bool),
        on_choose: impl Fn(TimeControl, &mut Window, &mut App) + 'static,
        on_rated: impl Fn(bool, &mut Window, &mut App) + 'static,
        (on_send, on_close): (
            impl Fn(&mut Window, &mut App) + 'static,
            impl Fn(&mut Window, &mut App) + 'static,
        ),
    ) -> Self {
        Self {
            choice,
            rated,
            can_rate,
            on_choose: Rc::new(on_choose),
            on_rated: Rc::new(on_rated),
            on_send: Rc::new(on_send),
            on_close: Rc::new(on_close),
        }
    }
}

impl RenderOnce for ChallengePicker {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (on_rated, on_send, on_close) = (
            self.on_rated.clone(),
            self.on_send.clone(),
            self.on_close.clone(),
        );
        v_flex()
            .gap_3()
            .p_3()
            .rounded(cx.theme().radius)
            .bg(cx.theme().muted)
            .children(Category::ALL.map(|category| self.speed_row(category)))
            .child(
                h_flex()
                    .justify_between()
                    .child(rated_label(self.can_rate, cx))
                    .child(
                        Switch::new("challenge-rated")
                            .checked(self.rated && self.can_rate)
                            .disabled(!self.can_rate)
                            .on_click(move |on, window, cx| on_rated(*on, window, cx)),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("send-challenge")
                            .primary()
                            .label("Send challenge")
                            .on_click(move |_, window, cx| on_send(window, cx)),
                    )
                    .child(
                        Button::new("close-challenge")
                            .ghost()
                            .label("Cancel")
                            .on_click(move |_, window, cx| on_close(window, cx)),
                    ),
            )
    }
}

impl ChallengePicker {
    fn speed_row(&self, category: Category) -> Div {
        h_flex()
            .gap_2()
            .child(div().w_20().text_sm().child(category.name()))
            .children(
                presets_in(category)
                    .into_iter()
                    .map(|control| self.preset_button(control)),
            )
    }

    /// Filled once picked, so the choice shows on the grey panel; outlined otherwise.
    fn preset_button(&self, control: TimeControl) -> Button {
        let on_choose = Rc::clone(&self.on_choose);
        let id = SharedString::from(format!("challenge-time-{}", control.label()));
        let button = Button::new(id)
            .small()
            .label(control.label())
            .on_click(move |_, window, cx| on_choose(control, window, cx));
        if self.choice == control {
            button.primary()
        } else {
            button.outline()
        }
    }
}

/// "Rated", and for a guest why it stays off.
fn rated_label(can_rate: bool, cx: &App) -> Div {
    v_flex()
        .child(Label::new("Rated").text_sm())
        .when(!can_rate, |this| {
            this.child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Guests play unrated games"),
            )
        })
}
