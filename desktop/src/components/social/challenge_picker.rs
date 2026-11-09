use std::rc::Rc;

use chess_domain::{Category, TimeControl};
use gpui_kit::{
    component::{
        ActiveTheme as _, Sizable as _,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    *,
};

use crate::{
    components::{
        panel::{ChoiceTile, RatedSwitch, StartButton},
        play::speed_label,
    },
    services::play::presets_in,
};

type OnChoose = Rc<dyn Fn(TimeControl, &mut Window, &mut App)>;
type OnRated = Rc<dyn Fn(bool, &mut Window, &mut App)>;
type OnPress = Rc<dyn Fn(&mut Window, &mut App)>;

/// What a challenge asks for, as the Play lobby asks: a time control, rated or not, then Send.
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
        let theme = cx.theme();
        let (on_rated, on_send, on_close) = (
            Rc::clone(&self.on_rated),
            Rc::clone(&self.on_send),
            Rc::clone(&self.on_close),
        );
        let [bullet, blitz, rapid, classical] = Category::ALL.map(|category| self.speed(category));
        v_flex()
            .w_full()
            .gap_4()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.muted_foreground)
                    .child("Pick a time control"),
            )
            // The four speeds two by two, as the Play lobby lays its time controls out.
            .child(h_flex().gap_4().child(bullet).child(blitz))
            .child(h_flex().gap_4().child(rapid).child(classical))
            .child(RatedSwitch::new(
                ("challenge-rated", "Guests play unrated games"),
                (self.rated, self.can_rate),
                move |on, window, cx| on_rated(on, window, cx),
            ))
            .child(
                h_flex()
                    .gap_3()
                    .child(div().flex_1().child(StartButton::new(
                        "send-challenge",
                        "Send challenge",
                        move |window, cx| on_send(window, cx),
                    )))
                    .child(
                        Button::new("close-challenge")
                            .ghost()
                            .large()
                            .label("Cancel")
                            .on_click(move |_, window, cx| on_close(window, cx)),
                    ),
            )
    }
}

impl ChallengePicker {
    /// One speed: its name and picture over its three time controls as tiles.
    fn speed(&self, category: Category) -> Div {
        v_flex()
            .flex_1()
            .min_w_0()
            .gap_1p5()
            .child(speed_label(category))
            .child(
                h_flex().gap_2().children(
                    presets_in(category)
                        .into_iter()
                        .map(|control| div().flex_1().child(self.preset_tile(control))),
                ),
            )
    }

    /// A time to pick, lit green once picked, as in the Play lobby.
    fn preset_tile(&self, control: TimeControl) -> ChoiceTile {
        let on_choose = Rc::clone(&self.on_choose);
        ChoiceTile::new(
            format!("challenge-time-{}", control.label()),
            control.label(),
            self.choice == control,
            move |window, cx| on_choose(control, window, cx),
        )
        .height(px(38.))
    }
}
