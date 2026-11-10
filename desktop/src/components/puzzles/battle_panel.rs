use std::{rc::Rc, time::Duration};

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _,
        button::{Button, ButtonVariants},
        h_flex, v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use super::puzzle_pictures::BATTLE_PICTURE;
use crate::{
    components::{
        panel::{ChoiceTile, PanelTitle, StartButton},
        play::ClockFace,
    },
    enums::puzzles::BattleResult,
    i18n::tr,
    services::puzzles::{rating_change_text, result_words},
};

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// Beside the board during a battle: the time left and the score, then how it ended.
#[derive(IntoElement)]
pub struct BattlePanel {
    left: Duration,
    /// Whether it moves the players' battle ratings.
    rated: bool,
    scores: (u32, u32),
    result: Option<(BattleResult, Option<i32>)>,
    on_again: OnClick,
    on_back: OnClick,
    on_resign: Option<OnClick>,
}

impl BattlePanel {
    pub fn new(
        (left, rated): (Duration, bool),
        scores: (u32, u32),
        result: Option<(BattleResult, Option<i32>)>,
        (on_again, on_back): (OnClick, OnClick),
    ) -> Self {
        Self {
            left,
            rated,
            scores,
            result,
            on_again,
            on_back,
            on_resign: None,
        }
    }

    /// Resign, while the battle is on.
    pub fn on_resign(mut self, on_resign: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_resign = Some(Rc::new(on_resign));
        self
    }
}

impl RenderOnce for BattlePanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (mine, theirs) = self.scores;
        let (on_again, on_back) = (self.on_again, self.on_back);
        let resign = self.on_resign.filter(|_| self.result.is_none());
        v_flex()
            .size_full()
            .items_center()
            .gap_4()
            .child(PanelTitle::new(BATTLE_PICTURE, tr!("puzzles.battle.title")))
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.muted_foreground)
                    .child(if self.rated {
                        tr!("puzzles.battle.rated_tag")
                    } else {
                        tr!("puzzles.battle.casual_tag")
                    }),
            )
            .child(ClockFace::new(self.left, self.result.is_none()))
            .child(
                h_flex()
                    .gap_4()
                    .items_center()
                    .child(score_column(tr!("puzzles.battle.you"), mine, cx))
                    .child(
                        div()
                            .text_2xl()
                            .text_color(theme.muted_foreground)
                            .child("–"),
                    )
                    .child(score_column(tr!("puzzles.battle.opponent"), theirs, cx)),
            )
            .when_some(resign, |this, on_resign| {
                this.child(div().flex_1()).child(
                    ChoiceTile::new(
                        "battle-resign",
                        tr!("puzzles.battle.resign"),
                        false,
                        move |window, cx| {
                            on_resign(window, cx);
                        },
                    )
                    .icon(IconName::Flag),
                )
            })
            .when_some(self.result, |this, (result, change)| {
                this.child(
                    div()
                        .text_2xl()
                        .font_weight(FontWeight::EXTRA_BOLD)
                        .child(result_words(result)),
                )
                .when_some(rating_change_text(change), |this, line| {
                    this.child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(line),
                    )
                })
                .child(div().flex_1())
                .child(StartButton::new(
                    "battle-again",
                    tr!("puzzles.battle.play_again"),
                    move |window, cx| {
                        on_again(window, cx);
                    },
                ))
                .child(
                    Button::new("battle-back")
                        .ghost()
                        .w_full()
                        .label(tr!("puzzles.battle.back"))
                        .on_click(move |_, window, cx| on_back(window, cx)),
                )
            })
    }
}

fn score_column(who: String, score: u32, cx: &App) -> Div {
    v_flex()
        .items_center()
        .child(
            div()
                .text_3xl()
                .font_weight(FontWeight::EXTRA_BOLD)
                .child(score.to_string()),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(who),
        )
}
