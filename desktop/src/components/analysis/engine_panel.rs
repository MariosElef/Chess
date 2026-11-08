use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, h_flex, switch::Switch, v_flex},
    *,
};

use chess_domain::Side;

use super::side_chip::side_chip;
use crate::{
    enums::analysis::EngineStatus,
    models::analysis::LineView,
    services::analysis::{favours_black, format_nps},
};

type OnToggle = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// Stockfish's switch, its status, and its best lines.
#[derive(IntoElement)]
pub struct EnginePanel {
    enabled: bool,
    status: EngineStatus,
    lines: Vec<LineView>,
    on_toggle: OnToggle,
}

impl EnginePanel {
    pub fn new(
        enabled: bool,
        status: EngineStatus,
        lines: Vec<LineView>,
        on_toggle: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            enabled,
            status,
            lines,
            on_toggle: Rc::new(on_toggle),
        }
    }
}

impl RenderOnce for EnginePanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_2()
            .p_3()
            .rounded(px(8.))
            .bg(cx.theme().muted)
            .child(self.header(cx))
            .children(self.lines.iter().map(line_row))
    }
}

impl EnginePanel {
    fn header(&self, cx: &App) -> Div {
        let on_toggle = Rc::clone(&self.on_toggle);
        h_flex()
            .gap_2()
            .child(
                Icon::from(IconName::Fish)
                    .size_5()
                    .text_color(cx.theme().blue),
            )
            .child(div().font_weight(FontWeight::BOLD).child("Stockfish"))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(status_text(self.status)),
            )
            .child(
                Switch::new("engine-switch")
                    .checked(self.enabled)
                    .on_click(move |on, window, cx| on_toggle(*on, window, cx)),
            )
    }
}

/// The evaluation on a chip, white when White is better and dark when Black is, then the
/// moves on one line.
fn line_row(line: &LineView) -> Div {
    let side = if favours_black(line.eval) {
        Side::Black
    } else {
        Side::White
    };
    h_flex()
        .gap_2()
        .text_sm()
        .child(
            side_chip(line.eval.label(), side)
                .w(px(56.))
                .flex_shrink_0()
                .py_0p5(),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .child(line.moves.clone()),
        )
}

fn status_text(status: EngineStatus) -> String {
    match status {
        EngineStatus::Off => "off".to_owned(),
        EngineStatus::Starting => "starting…".to_owned(),
        EngineStatus::Running { depth, nps } => format!("depth {depth} · {}", format_nps(nps)),
        EngineStatus::Finished => "game over".to_owned(),
        EngineStatus::PausedForReview => "paused for the review".to_owned(),
        EngineStatus::NotFound => "not found: brew install stockfish".to_owned(),
        EngineStatus::Stopped => "stopped: switch it on to restart".to_owned(),
    }
}
