use std::{rc::Rc, time::Duration};

use chess_domain::Side;
use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use crate::components::{
    panel::{PanelTitle, StartButton},
    play::format_clock,
    puzzles::status_card,
    shell::PUZZLES_PICTURE,
};

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// Practice's panel: a status card, the player's rating, the difficulty, the puzzle, and Next.
#[derive(IntoElement)]
pub struct PuzzlePanel {
    status: String,
    /// The status is good news, shown in the theme's success colour.
    status_good: bool,
    /// The side the player solves for, shown beside the status.
    side: Option<Side>,
    rating: Option<u32>,
    themes: Vec<String>,
    elapsed: Option<Duration>,
    problem: Option<String>,
    /// The player's puzzle rating after the last attempt, with its change.
    player_rating: Option<String>,
    /// The difficulty slider, under the rating.
    difficulty: Option<AnyElement>,
    loading: bool,
    on_next: OnClick,
    on_back: OnClick,
}

impl PuzzlePanel {
    pub fn new((status, status_good): (String, bool), side: Option<Side>) -> Self {
        Self {
            status,
            status_good,
            side,
            rating: None,
            themes: Vec::new(),
            elapsed: None,
            problem: None,
            player_rating: None,
            difficulty: None,
            loading: false,
            on_next: Rc::new(|_, _| {}),
            on_back: Rc::new(|_, _| {}),
        }
    }

    /// The puzzle's own rating and themes.
    pub fn puzzle(mut self, rating: Option<u32>, themes: Vec<String>) -> Self {
        self.rating = rating;
        self.themes = themes;
        self
    }

    /// How long the player has been on it.
    pub fn elapsed(mut self, elapsed: Option<Duration>) -> Self {
        self.elapsed = elapsed;
        self
    }

    pub fn problem(mut self, problem: Option<String>) -> Self {
        self.problem = problem;
        self
    }

    pub fn player_rating(mut self, rating: Option<String>) -> Self {
        self.player_rating = rating;
        self
    }

    /// The difficulty slider, for puzzles at a picked difficulty; rated ones have none.
    pub fn difficulty(mut self, picker: Option<impl IntoElement>) -> Self {
        self.difficulty = picker.map(IntoElement::into_any_element);
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn on_next(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_next = Rc::new(handler);
        self
    }

    pub fn on_back(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_back = Rc::new(handler);
        self
    }
}

impl RenderOnce for PuzzlePanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (on_next, on_back) = (self.on_next, self.on_back);
        let next_label = if self.loading {
            "Finding a puzzle…"
        } else {
            "Next puzzle"
        };
        v_flex()
            .size_full()
            .gap_3()
            .child(
                PanelTitle::new(PUZZLES_PICTURE, "Puzzles").on_back(move |window, cx| {
                    on_back(window, cx);
                }),
            )
            .child(status_card(&self.status, self.status_good, self.side, cx))
            .when_some(self.player_rating, |this, rating| {
                this.child(
                    v_flex()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .text_color(theme.muted_foreground)
                                .child("YOUR RATING"),
                        )
                        .child(
                            div()
                                .text_2xl()
                                .font_weight(FontWeight::EXTRA_BOLD)
                                .child(rating),
                        ),
                )
            })
            .children(self.difficulty)
            .when_some(self.problem, |this, problem| {
                this.child(div().text_sm().text_color(theme.danger).child(problem))
            })
            .child(div().flex_1())
            .when(!self.themes.is_empty(), |this| {
                this.child(themes(&self.themes, cx))
            })
            .child(
                h_flex()
                    .justify_between()
                    .text_sm()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.muted_foreground)
                    .child(
                        h_flex()
                            .gap_1p5()
                            .child(Icon::from(IconName::Signal).size_4())
                            .child(
                                self.rating
                                    .map_or_else(String::new, |rating| rating.to_string()),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .child(Icon::from(IconName::Timer).size_4())
                            .child(format_clock(self.elapsed.unwrap_or_default())),
                    ),
            )
            .child(
                StartButton::new("next-puzzle", next_label, move |window, cx| {
                    on_next(window, cx);
                })
                .busy(self.loading),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child("Puzzles from lichess.org, free for everyone (CC0)"),
            )
    }
}

/// The puzzle's themes as small tags: mateIn3, sacrifice, endgame…
fn themes(themes: &[String], cx: &App) -> Div {
    h_flex()
        .flex_wrap()
        .gap_1()
        .children(themes.iter().map(|theme| {
            div()
                .px_2()
                .py_0p5()
                .rounded(cx.theme().radius)
                .bg(cx.theme().secondary)
                .text_xs()
                .child(theme.clone())
        }))
}
