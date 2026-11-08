use std::rc::Rc;

use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, h_flex, label::Label, switch::Switch, v_flex},
    *,
};

use crate::{
    enums::analysis::EngineStatus, models::analysis::LineView, services::analysis::format_nps,
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
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(self.header(cx))
            .children(self.lines.iter().map(|line| line_row(line, cx)))
    }
}

impl EnginePanel {
    fn header(&self, cx: &App) -> Div {
        let on_toggle = Rc::clone(&self.on_toggle);
        h_flex()
            .gap_2()
            .child(Icon::from(IconName::Cpu))
            .child(Label::new("Stockfish").text_sm())
            .child(
                div()
                    .flex_1()
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

/// The evaluation in a small box, then the moves on one line.
fn line_row(line: &LineView, cx: &App) -> Div {
    h_flex()
        .gap_2()
        .text_sm()
        .child(
            div()
                .w_12()
                .flex_shrink_0()
                .px_1()
                .rounded(cx.theme().radius)
                .bg(cx.theme().muted)
                .text_center()
                .font_family(cx.theme().mono_font_family.clone())
                .child(line.eval.label()),
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
        EngineStatus::NotFound => "not found — install it with: brew install stockfish".to_owned(),
        EngineStatus::Stopped => "stopped — switch it off and on to restart".to_owned(),
    }
}
