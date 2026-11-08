use std::rc::Rc;

use chess_domain::{Side, engine::Eval};
use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme as _, Icon, h_flex, label::Label, switch::Switch, v_flex},
    *,
};

/// What the engine is doing, as the panel's header says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineStatus {
    Off,
    Starting,
    Running {
        depth: u32,
        nps: u64,
    },
    /// The position is over: nothing to search.
    Finished,
    NotFound,
    Stopped,
}

/// One of the engine's lines, ready to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineView {
    pub eval: Eval,
    pub moves: String,
}

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

/// Nodes per second, short: `850 kn/s`, `1.2 Mn/s`.
pub fn format_nps(nps: u64) -> String {
    if nps >= 1_000_000 {
        format!("{}.{} Mn/s", nps / 1_000_000, nps % 1_000_000 / 100_000)
    } else {
        format!("{} kn/s", nps / 1_000)
    }
}

/// Moves with their numbers, starting from the position's move number and side:
/// `12. e4 e5 13. Nf3`, or `12… Nf6 13. Bd3` when Black moves first.
pub fn numbered_line(fullmove: u32, side_to_move: Side, sans: &[String]) -> String {
    let mut number = fullmove;
    let mut side = side_to_move;
    let mut words = Vec::new();
    for (index, san) in sans.iter().enumerate() {
        match (side, index) {
            (Side::White, _) => words.push(format!("{number}. {san}")),
            (Side::Black, 0) => words.push(format!("{number}… {san}")),
            (Side::Black, _) => words.push(san.clone()),
        }
        if side == Side::Black {
            number += 1;
        }
        side = side.opponent();
    }
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use chess_domain::Side;

    use super::{format_nps, numbered_line};

    fn sans(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn white_lines_number_every_pair() {
        assert_eq!(
            numbered_line(1, Side::White, &sans("e4 e5 Nf3")),
            "1. e4 e5 2. Nf3"
        );
    }

    #[test]
    fn black_lines_start_with_an_ellipsis() {
        assert_eq!(
            numbered_line(12, Side::Black, &sans("Nf6 Bd3 O-O")),
            "12… Nf6 13. Bd3 O-O"
        );
    }

    #[test]
    fn speeds_read_in_thousands_or_millions() {
        assert_eq!(format_nps(625_500), "625 kn/s");
        assert_eq!(format_nps(1_250_000), "1.2 Mn/s");
    }
}
