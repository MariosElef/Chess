use gpui_kit::{
    component::{ActiveTheme as _, h_flex, label::Label},
    prelude::FluentBuilder as _,
    *,
};

use crate::{
    components::social::strip_avatar,
    models::puzzles::BattleSide,
    services::{puzzles::STRIKES, social::face_of},
};

/// A player's battle bar: picture, name and rating, a mark per mistake, and their score.
#[derive(IntoElement)]
pub struct BattleBar {
    side: BattleSide,
}

impl BattleBar {
    pub fn new(side: BattleSide) -> Self {
        Self { side }
    }
}

impl RenderOnce for BattleBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let name = match self.side.rating {
            Some(rating) => format!("{} ({rating})", self.side.username),
            None => self.side.username.clone(),
        };
        let marks = (0..STRIKES).map(|strike| {
            let made = strike < self.side.mistakes;
            div()
                .size_3()
                .rounded_full()
                .when(made, |this| this.bg(theme.red))
                .when(!made, |this| this.bg(theme.muted))
        });
        h_flex()
            .w_full()
            .justify_between()
            .gap_4()
            .child(
                h_flex()
                    .gap_3()
                    .child(strip_avatar(
                        &face_of(
                            &self.side.id,
                            &self.side.username,
                            self.side.avatar.as_deref(),
                        ),
                        cx,
                    ))
                    .child(Label::new(name)),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(h_flex().gap_1().children(marks))
                    .child(
                        div()
                            .min_w_12()
                            .px_3()
                            .py_1()
                            .rounded(theme.radius)
                            .bg(theme.muted)
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .text_center()
                            .child(self.side.score.to_string()),
                    ),
            )
    }
}
