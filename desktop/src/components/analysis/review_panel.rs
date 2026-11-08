use gpui_kit::{
    component::{ActiveTheme as _, h_flex, label::Label, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use crate::enums::analysis::MoveClass;

/// The move being looked at, as the review sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownVerdict {
    pub class: MoveClass,
    pub san: String,
    /// The engine's choice, written out, when the move was not it.
    pub best: Option<String>,
}

/// A finished review: each side's accuracy, how many of each kind of move they made, and what
/// the move on the board was.
#[derive(IntoElement)]
pub struct ReviewPanel {
    /// White's, then Black's.
    accuracy: [f64; 2],
    /// The rating each side played like; `None` with too few moves to go on.
    ratings: [Option<u32>; 2],
    /// Each class with White's count and Black's.
    counts: Vec<(MoveClass, usize, usize)>,
    shown: Option<ShownVerdict>,
}

impl ReviewPanel {
    pub fn new(
        (accuracy, ratings): ([f64; 2], [Option<u32>; 2]),
        counts: Vec<(MoveClass, usize, usize)>,
        shown: Option<ShownVerdict>,
    ) -> Self {
        Self {
            accuracy,
            ratings,
            counts,
            shown,
        }
    }
}

impl RenderOnce for ReviewPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        v_flex()
            .w_full()
            .gap_3()
            .p_3()
            .rounded(cx.theme().radius)
            .bg(cx.theme().muted)
            .text_sm()
            .child(Label::new("Game review"))
            .when_some(self.shown, |this, shown| this.child(verdict(shown, muted)))
            .child(accuracy_row(self.accuracy, muted))
            .child(rating_row(self.ratings, muted))
            .children(
                self.counts
                    .into_iter()
                    .map(|(class, white, black)| count_row(class, white, black)),
            )
    }
}

/// `Nf3 is a Blunder`, with what was best below.
fn verdict(shown: ShownVerdict, muted: Hsla) -> Div {
    let line = format!("{} is {}", shown.san, shown.class.verdict());
    v_flex()
        .gap_1()
        .child(
            h_flex()
                .gap_2()
                .child(img(shown.class.icon_path()).size_5())
                .child(line),
        )
        .when_some(shown.best, |this, best| {
            this.child(div().text_color(muted).child(format!("Best was {best}")))
        })
}

fn accuracy_row(accuracy: [f64; 2], muted: Hsla) -> Div {
    h_flex()
        .justify_between()
        .child(div().text_color(muted).child("Accuracy"))
        .child(
            h_flex()
                .gap_4()
                .child(format!("White {:.1}", accuracy[0]))
                .child(format!("Black {:.1}", accuracy[1])),
        )
}

/// The rating each side played like, as an estimate: `~1850`, or a dash with too few moves.
fn rating_row(ratings: [Option<u32>; 2], muted: Hsla) -> Div {
    let estimate = |rating: Option<u32>| rating.map_or_else(|| "—".to_owned(), |r| format!("~{r}"));
    h_flex()
        .justify_between()
        .child(div().text_color(muted).child("Game rating"))
        .child(
            h_flex()
                .gap_4()
                .child(format!("White {}", estimate(ratings[0])))
                .child(format!("Black {}", estimate(ratings[1]))),
        )
}

fn count_row(class: MoveClass, white: usize, black: usize) -> Div {
    h_flex()
        .gap_2()
        .child(img(class.icon_path()).size_4())
        .child(div().flex_1().child(class.label()))
        .child(div().w_8().text_right().child(white.to_string()))
        .child(div().w_8().text_right().child(black.to_string()))
}
