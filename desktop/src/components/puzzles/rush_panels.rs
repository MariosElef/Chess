use std::{rc::Rc, time::Duration};

use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, button::Button, h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use super::status_card::status_card;
use crate::{
    components::panel::{StartButton, StatCard},
    enums::puzzles::RushKind,
    models::puzzles::RushRecord,
    services::puzzles::STRIKES,
};

type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

/// During a rush: the clock and the strikes, and how the puzzle is going. The score shows large
/// under the player's picture, beside it.
#[derive(IntoElement)]
pub struct RushScore {
    strikes: u32,
    time_left: Option<Duration>,
    status: String,
    status_good: bool,
    /// The next puzzle could not be fetched: why, and a way to ask again.
    problem: Option<(String, OnClick)>,
}

impl RushScore {
    pub fn new(
        strikes: u32,
        time_left: Option<Duration>,
        (status, status_good): (String, bool),
    ) -> Self {
        Self {
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
        let on_retry: OnClick = Rc::new(on_retry);
        self.problem = problem.map(|problem| (problem, on_retry));
        self
    }
}

impl RenderOnce for RushScore {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let time = self
            .time_left
            .map_or_else(|| "∞".to_owned(), minutes_seconds);
        panel()
            .child(status_card(&self.status, self.status_good, None, cx))
            .child(h_flex().gap_3().child(StatCard::new(
                (IconName::Timer, theme.green),
                time,
                "Time left",
            )))
            .child(
                h_flex()
                    .justify_center()
                    .child(strike_marks(self.strikes, cx)),
            )
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
    on_again: OnClick,
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
        panel()
            .items_center()
            .child(muted(&format!("{} rush over", self.kind.label()), cx))
            .child(
                div()
                    .text_size(px(48.))
                    .font_weight(FontWeight::EXTRA_BOLD)
                    .child(self.score.to_string()),
            )
            .when_some(self.record, |this, record| {
                let line = if record.new_best {
                    "New best!".to_owned()
                } else {
                    format!("Best {}", record.best)
                };
                this.child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(cx.theme().success)
                        .child(line),
                )
            })
            .child(div().h_4())
            .child(StartButton::new(
                "rush-again",
                "Play again",
                move |window, cx| {
                    on_again(window, cx);
                },
            ))
    }
}

fn panel() -> Div {
    v_flex().w_full().gap_4()
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
        .text_2xl()
        .font_weight(FontWeight::BOLD)
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
