use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, h_flex, v_flex},
    *,
};

use crate::models::puzzles::PuzzleOutcome;

/// The puzzles of a run or a session in order, as chess.com shows them: a green tick or a red
/// cross for each, with the puzzle's rating under it, wrapping onto new lines as they add up.
#[derive(IntoElement)]
pub struct SessionResults {
    outcomes: Vec<PuzzleOutcome>,
}

impl SessionResults {
    pub fn new(outcomes: Vec<PuzzleOutcome>) -> Self {
        Self { outcomes }
    }
}

impl RenderOnce for SessionResults {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex().w_full().flex_wrap().gap_x_2().gap_y_3().children(
            self.outcomes
                .iter()
                .map(|outcome| result_chip(*outcome, cx)),
        )
    }
}

/// One puzzle: its mark in a small coloured square, its rating under it in the same colour.
fn result_chip(outcome: PuzzleOutcome, cx: &App) -> Div {
    let theme = cx.theme();
    let (colour, icon) = if outcome.solved {
        (theme.green, IconName::Check)
    } else {
        (theme.danger, IconName::Close)
    };
    v_flex()
        .items_center()
        .gap_0p5()
        .child(
            div()
                .size_4()
                .rounded(px(3.))
                .flex()
                .items_center()
                .justify_center()
                .bg(colour)
                .child(Icon::new(icon).size_3().text_color(white())),
        )
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .text_color(colour)
                .child(outcome.rating.to_string()),
        )
}
