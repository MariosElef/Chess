use std::time::Duration;

use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, h_flex, label::Label},
    prelude::FluentBuilder as _,
    *,
};

use super::clock_face::ClockFace;
use crate::{
    components::{board::PLAYER_BAR, social::strip_avatar},
    models::account::Face,
};

/// A player's bar before the game: their picture, their name, and the clock they will start on.
#[derive(IntoElement)]
pub struct WaitingBar {
    name: String,
    /// A bot's picture, from the app's own.
    picture: Option<&'static str>,
    /// A player's picture, or their initial: `None` for one not known yet, as an opponent.
    person: Option<Face>,
    clock: Option<Duration>,
}

impl WaitingBar {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            picture: None,
            person: None,
            clock: None,
        }
    }

    pub fn picture(mut self, picture: Option<&'static str>) -> Self {
        self.picture = picture;
        self
    }

    /// A player's face, their picture or their initial; none while not known, as signed out.
    pub fn person(mut self, face: Option<Face>) -> Self {
        self.person = face;
        self
    }

    pub fn clock(mut self, clock: Option<Duration>) -> Self {
        self.clock = clock;
        self
    }
}

impl RenderOnce for WaitingBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let picture = match (self.picture, self.person) {
            (Some(path), _) => img(path).size_8().rounded(px(4.)).into_any_element(),
            (None, Some(face)) => strip_avatar(&face, cx).into_any_element(),
            (None, None) => div()
                .size_8()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .bg(cx.theme().muted_foreground)
                .child(Icon::from(IconName::User).text_color(cx.theme().background))
                .into_any_element(),
        };
        h_flex()
            .w_full()
            .h(PLAYER_BAR)
            .flex_shrink_0()
            .justify_between()
            .gap_4()
            .child(h_flex().gap_3().child(picture).child(Label::new(self.name)))
            .when_some(self.clock, |this, left| {
                this.child(ClockFace::new(left, false))
            })
    }
}
