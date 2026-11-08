use std::time::Duration;

use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, h_flex, label::Label},
    prelude::FluentBuilder as _,
    *,
};

use super::clock_face::ClockFace;

/// Every player bar's height, clock or not, so the board is the same size either way.
pub const PLAYER_BAR: Pixels = px(40.);

/// A player's bar before the game: their picture, their name, and the clock they will start
/// on, if the game has one.
#[derive(IntoElement)]
pub struct WaitingBar {
    name: String,
    picture: Option<&'static str>,
    clock: Option<Duration>,
}

impl WaitingBar {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            picture: None,
            clock: None,
        }
    }

    pub fn picture(mut self, picture: Option<&'static str>) -> Self {
        self.picture = picture;
        self
    }

    pub fn clock(mut self, clock: Option<Duration>) -> Self {
        self.clock = clock;
        self
    }
}

impl RenderOnce for WaitingBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let picture = match self.picture {
            Some(path) => img(path).size_8().rounded(px(4.)).into_any_element(),
            None => div()
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
