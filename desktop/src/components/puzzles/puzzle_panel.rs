use std::rc::Rc;

use gpui_kit::{
    component::{
        ActiveTheme as _,
        button::{Button, ButtonVariants},
        h_flex,
        label::Label,
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

type OnNext = Rc<dyn Fn(&mut Window, &mut App)>;

/// Beside the puzzle's board: its rating, how it is going, its themes, and the next one.
#[derive(IntoElement)]
pub struct PuzzlePanel {
    rating: Option<u32>,
    status: String,
    /// The status is good news, shown in the theme's success colour.
    status_good: bool,
    themes: Vec<String>,
    problem: Option<String>,
    /// The player's puzzle rating after the last attempt, with its change.
    player_rating: Option<String>,
    loading: bool,
    on_next: OnNext,
}

impl PuzzlePanel {
    pub fn new(
        rating: Option<u32>,
        status: String,
        status_good: bool,
        themes: Vec<String>,
    ) -> Self {
        Self {
            rating,
            status,
            status_good,
            themes,
            problem: None,
            player_rating: None,
            loading: false,
            on_next: Rc::new(|_, _| {}),
        }
    }

    pub fn problem(mut self, problem: Option<String>) -> Self {
        self.problem = problem;
        self
    }

    pub fn player_rating(mut self, rating: Option<String>) -> Self {
        self.player_rating = rating;
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
}

impl RenderOnce for PuzzlePanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_next = self.on_next;
        let status_color = if self.status_good {
            cx.theme().success
        } else {
            cx.theme().foreground
        };
        v_flex()
            .w_80()
            .flex_shrink_0()
            .gap_4()
            .p_4()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .justify_between()
                    .child(Label::new("Puzzle").text_lg())
                    .when_some(self.rating, |this, rating| {
                        this.child(
                            Label::new(format!("Rating {rating}"))
                                .text_sm()
                                .text_color(cx.theme().muted_foreground),
                        )
                    }),
            )
            .child(div().text_color(status_color).child(self.status))
            .when_some(self.player_rating, |this, rating| {
                this.child(div().text_sm().child(format!("Your rating {rating}")))
            })
            .when_some(self.problem, |this, problem| {
                this.child(div().text_sm().text_color(cx.theme().danger).child(problem))
            })
            .when(!self.themes.is_empty(), |this| {
                this.child(themes(&self.themes, cx))
            })
            .child(
                Button::new("next-puzzle")
                    .primary()
                    .w_full()
                    .label("Next puzzle")
                    .loading(self.loading)
                    .on_click(move |_, window, cx| on_next(window, cx)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
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
