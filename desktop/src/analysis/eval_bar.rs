#![expect(
    clippy::float_arithmetic,
    reason = "the bar splits its height by the evaluation's share"
)]

use chess_domain::engine::Eval;
use gpui_kit::{
    component::{ActiveTheme as _, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use crate::board::BoardColors;

/// A level bar while nothing is known yet.
const LEVEL: f64 = 0.5;

/// White's winning chances as a vertical bar beside the board, White's part on White's side.
#[derive(IntoElement)]
pub struct EvalBar {
    eval: Option<Eval>,
    /// The board is flipped, so White is at the top.
    flipped: bool,
}

impl EvalBar {
    pub fn new(eval: Option<Eval>, flipped: bool) -> Self {
        Self { eval, flipped }
    }
}

impl RenderOnce for EvalBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = BoardColors::current(cx);
        let layout = layout(self.eval, self.flipped);
        let (top, bottom) = if layout.top_is_white {
            (colors.eval_white, colors.eval_black)
        } else {
            (colors.eval_black, colors.eval_white)
        };
        v_flex()
            .relative()
            .w_6()
            .h_full()
            .flex_shrink_0()
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .bg(bottom)
            .child(div().w_full().h(relative(layout.top_share)).bg(top))
            .when_some(self.eval, |this, eval| {
                this.child(label(eval, layout.label_on_top, colors))
            })
    }
}

/// How the bar is split: one decision, so colour and share can never flip separately.
#[derive(Debug, Clone, Copy, PartialEq)]
struct BarLayout {
    top_is_white: bool,
    top_share: f32,
    /// The number sits at the end of the side that is ahead.
    label_on_top: bool,
}

fn layout(eval: Option<Eval>, flipped: bool) -> BarLayout {
    let white = eval.map_or(LEVEL, Eval::white_share);
    let top_is_white = flipped;
    let top_share = if top_is_white { white } else { 1.0 - white };
    let white_ahead = white >= LEVEL;
    BarLayout {
        top_is_white,
        top_share: top_share as f32,
        label_on_top: white_ahead == top_is_white,
    }
}

/// The evaluation in small type, dark on White's part and light on Black's.
fn label(eval: Eval, on_top: bool, colors: BoardColors) -> Div {
    let on_white = eval.white_share() >= LEVEL;
    div()
        .absolute()
        .left_0()
        .right_0()
        .when(on_top, |this| this.top_1())
        .when(!on_top, |this| this.bottom_1())
        .text_center()
        .text_size(rems(0.6))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(if on_white {
            colors.eval_black
        } else {
            colors.eval_white
        })
        .child(eval.label())
}

#[cfg(test)]
mod tests {
    use chess_domain::{Side, engine::Eval};

    use super::layout;

    #[test]
    fn white_sits_at_the_bottom_unless_flipped() {
        let level = layout(None, false);
        assert!(!level.top_is_white);
        assert!((level.top_share - 0.5).abs() < 1e-6);
        assert!(layout(None, true).top_is_white);
    }

    #[test]
    fn a_white_advantage_shrinks_black_at_the_top() {
        let ahead = layout(Some(Eval::Centipawns(300)), false);
        assert!(ahead.top_share < 0.5);
        assert!(!ahead.label_on_top, "the number sits on White's end");
    }

    #[test]
    fn flipping_moves_white_and_its_share_together() {
        let eval = Some(Eval::Centipawns(300));
        let normal = layout(eval, false);
        let flipped = layout(eval, true);
        assert!((normal.top_share + flipped.top_share - 1.0).abs() < 1e-6);
        assert!(flipped.label_on_top);
    }

    #[test]
    fn a_mate_fills_the_bar() {
        let mate = layout(
            Some(Eval::Mate {
                winner: Side::Black,
                moves: 2,
            }),
            false,
        );
        assert!((mate.top_share - 1.0).abs() < 1e-6);
        assert!(mate.label_on_top);
    }
}
