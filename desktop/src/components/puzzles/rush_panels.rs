use std::{rc::Rc, time::Duration};

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

use crate::{enums::puzzles::RushKind, models::puzzles::RushRecord};

type OnAgain = Rc<dyn Fn(&mut Window, &mut App)>;
type OnRetry = Rc<dyn Fn(&mut Window, &mut App)>;

/// The mistakes a rush allows before it ends.
const STRIKES: u32 = 3;

/// During a rush: the score, the clock and the strikes, and how the puzzle is going.
#[derive(IntoElement)]
pub struct RushScore {
    score: u32,
    strikes: u32,
    time_left: Option<Duration>,
    status: String,
    status_good: bool,
    /// The next puzzle could not be fetched: why, and a way to ask again.
    problem: Option<(String, OnRetry)>,
}

impl RushScore {
    pub fn new(
        score: u32,
        strikes: u32,
        time_left: Option<Duration>,
        (status, status_good): (String, bool),
    ) -> Self {
        Self {
            score,
            strikes,
            time_left,
            status,
            status_good,
            problem: None,
        }
    }

    pub fn problem(
        mut self,
        problem: Option<String>,
        on_retry: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        let on_retry: OnRetry = Rc::new(on_retry);
        self.problem = problem.map(|problem| (problem, on_retry));
        self
    }
}

impl RenderOnce for RushScore {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let status_color = if self.status_good {
            cx.theme().success
        } else {
            cx.theme().foreground
        };
        panel(cx)
            .child(
                h_flex()
                    .justify_between()
                    .child(Label::new(self.score.to_string()).text_3xl())
                    .when_some(self.time_left, |this, left| {
                        this.child(Label::new(minutes_seconds(left)).text_xl())
                    }),
            )
            .child(strike_marks(self.strikes, cx))
            .child(div().text_color(status_color).child(self.status))
            .when_some(self.problem, |this, (problem, on_retry)| {
                this.child(div().text_sm().text_color(cx.theme().danger).child(problem))
                    .child(
                        Button::new("rush-retry")
                            .outline()
                            .w_full()
                            .label("Try again")
                            .on_click(move |_, window, cx| on_retry(window, cx)),
                    )
            })
    }
}

/// After a rush: the score, and another go.
#[derive(IntoElement)]
pub struct RushResult {
    score: u32,
    kind: RushKind,
    /// The server's best of this kind, once it has answered.
    record: Option<RushRecord>,
    on_again: OnAgain,
}

impl RushResult {
    pub fn new(
        score: u32,
        kind: RushKind,
        record: Option<RushRecord>,
        on_again: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            score,
            kind,
            record,
            on_again: Rc::new(on_again),
        }
    }
}

impl RenderOnce for RushResult {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_again = self.on_again;
        panel(cx)
            .child(muted(&format!("{} rush over", self.kind.label()), cx))
            .child(Label::new(format!("Score {}", self.score)).text_3xl())
            .when_some(self.record, |this, record| {
                let line = if record.new_best {
                    "New best!".to_owned()
                } else {
                    format!("Best {}", record.best)
                };
                this.child(div().text_color(cx.theme().success).child(line))
            })
            .child(
                Button::new("rush-again")
                    .primary()
                    .w_full()
                    .label("Play again")
                    .on_click(move |_, window, cx| on_again(window, cx)),
            )
    }
}

fn panel(cx: &App) -> Div {
    v_flex()
        .w_80()
        .flex_shrink_0()
        .gap_4()
        .p_4()
        .rounded(cx.theme().radius)
        .border_1()
        .border_color(cx.theme().border)
}

fn muted(text: &str, cx: &App) -> Div {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(text.to_owned())
}

/// One ✕ per mistake, in the danger colour; the strikes still left are dim.
fn strike_marks(strikes: u32, cx: &App) -> Div {
    h_flex()
        .gap_2()
        .text_xl()
        .children((0..STRIKES).map(|index| {
            let color = if index < strikes {
                cx.theme().danger
            } else {
                cx.theme().muted
            };
            div().text_color(color).child("✕")
        }))
}

/// `2:05`.
fn minutes_seconds(left: Duration) -> String {
    let seconds = left.as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::minutes_seconds;

    #[test]
    fn the_clock_reads_minutes_and_seconds() {
        assert_eq!(minutes_seconds(Duration::from_secs(125)), "2:05");
        assert_eq!(minutes_seconds(Duration::from_millis(9_900)), "0:09");
    }
}
