use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{
        ActiveTheme as _, Sizable as _,
        button::{Button, ButtonVariants},
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

use crate::components::{
    analysis::REVIEW_PICTURE,
    panel::{ChoiceTile, StartButton, action_bar},
};

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// What to do once a game is over: Game Review, large and green, then Analyze, New game, Rematch.
#[derive(IntoElement, Clone)]
pub struct GameOverActions {
    /// Ids are prefixed, so the card's and the panel's buttons are told apart.
    prefix: &'static str,
    on_review: OnClick,
    choices: Vec<(SharedString, IconName, OnClick)>,
}

impl GameOverActions {
    pub fn new(on_review: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        Self {
            prefix: "",
            on_review: Rc::new(on_review),
            choices: Vec::new(),
        }
    }

    /// A smaller button under Game Review, its label fixed or made at the time, such as "3 | 2".
    pub fn choice(
        mut self,
        label: impl Into<SharedString>,
        icon: IconName,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.choices.push((label.into(), icon, Rc::new(on_click)));
        self
    }

    fn prefixed(mut self, prefix: &'static str) -> Self {
        self.prefix = prefix;
        self
    }
}

impl RenderOnce for GameOverActions {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (prefix, on_review) = (self.prefix, self.on_review);
        let choices = self
            .choices
            .into_iter()
            .map(|(label, icon, on_click)| {
                ChoiceTile::new(
                    format!("{prefix}{label}"),
                    label,
                    false,
                    move |window, cx| {
                        on_click(window, cx);
                    },
                )
                .icon(icon)
            })
            .collect();
        v_flex()
            .w_full()
            .gap_2()
            .child(
                StartButton::new(
                    format!("{prefix}game-review"),
                    "Game Review",
                    move |window, cx| {
                        on_review(window, cx);
                    },
                )
                .picture(REVIEW_PICTURE),
            )
            .child(action_bar(choices))
    }
}

/// The card over the board when a game ends: who won and how, then the panel's actions.
#[derive(IntoElement)]
pub struct GameOverCard {
    title: String,
    reason: String,
    /// A line under the reason, such as the rating change.
    detail: Option<String>,
    actions: GameOverActions,
    on_close: OnClick,
}

impl GameOverCard {
    pub fn new(
        (title, reason): (String, String),
        actions: GameOverActions,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            title,
            reason,
            detail: None,
            actions: actions.prefixed("card-"),
            on_close: Rc::new(on_close),
        }
    }

    pub fn detail(mut self, detail: Option<String>) -> Self {
        self.detail = detail;
        self
    }
}

impl RenderOnce for GameOverCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let on_close = self.on_close;
        v_flex()
            .id("game-over")
            .debug_selector(|| "game-over".to_owned())
            .w(px(340.))
            .rounded(px(10.))
            .overflow_hidden()
            .bg(theme.popover)
            .shadow_lg()
            // A click on the card stays on it, so the board under it does not take it.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                v_flex()
                    .relative()
                    .items_center()
                    .gap_1()
                    .px_6()
                    .py_5()
                    .bg(theme.muted)
                    .child(
                        div()
                            .absolute()
                            .top_2()
                            .right_2()
                            .debug_selector(|| "close-game-over".to_owned())
                            .child(
                                Button::new("close-game-over")
                                    .ghost()
                                    .small()
                                    .icon(IconName::Close)
                                    .on_click(move |_, window, cx| on_close(window, cx)),
                            ),
                    )
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::EXTRA_BOLD)
                            .text_center()
                            .child(self.title),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(self.reason),
                    )
                    .when_some(self.detail, |this, detail| {
                        this.child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(detail),
                        )
                    }),
            )
            .child(div().p_4().child(self.actions))
    }
}

/// The board with the card centred over it.
pub fn with_card(board: impl IntoElement, card: Option<GameOverCard>) -> Div {
    div()
        .relative()
        .size_full()
        .child(board)
        .when_some(card, |this, card| {
            this.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(card),
            )
        })
}
