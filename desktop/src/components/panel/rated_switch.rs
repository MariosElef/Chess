use std::rc::Rc;

use gpui_kit::{
    component::{ActiveTheme as _, Disableable as _, h_flex, label::Label, switch::Switch, v_flex},
    prelude::FluentBuilder as _,
    *,
};

type OnRated = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// Rated or not; for a guest the switch stays off, with the reason under its name.
#[derive(IntoElement)]
pub struct RatedSwitch {
    id: &'static str,
    rated: bool,
    /// Signed in with an account: a guest never plays rated.
    can_rate: bool,
    /// Why a guest cannot, under the switch's name.
    guest_note: &'static str,
    on_rated: OnRated,
}

impl RatedSwitch {
    pub fn new(
        (id, guest_note): (&'static str, &'static str),
        (rated, can_rate): (bool, bool),
        on_rated: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id,
            rated,
            can_rate,
            guest_note,
            on_rated: Rc::new(on_rated),
        }
    }
}

impl RenderOnce for RatedSwitch {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_rated = self.on_rated;
        h_flex()
            .justify_between()
            .child(
                v_flex()
                    .child(Label::new("Rated").text_sm())
                    .when(!self.can_rate, |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(self.guest_note),
                        )
                    }),
            )
            .child(
                Switch::new(self.id)
                    .checked(self.rated && self.can_rate)
                    .disabled(!self.can_rate)
                    .on_click(move |on, window, cx| on_rated(*on, window, cx)),
            )
    }
}
