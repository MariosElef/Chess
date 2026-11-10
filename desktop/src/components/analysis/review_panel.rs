use chess_domain::{Piece, PieceKind, Side};
use gpui_kit::{
    component::{ActiveTheme as _, h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use super::{eval_graph::EvalGraph, side_chip::side_chip};
use crate::{
    components::{
        board::{BoardStyle, piece_image},
        play::FERRIS_PICTURE,
    },
    enums::analysis::MoveClass,
    i18n::tr,
    models::analysis::Opening,
    services::board::side_name,
};

/// The move being looked at, as the review sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownVerdict {
    pub class: MoveClass,
    pub san: String,
    /// The engine's choice, written out, when the move was not it.
    pub best: Option<String>,
}

/// A finished review: the evaluation graph, then each side's accuracy, move kinds and rating.
#[derive(IntoElement)]
pub struct ReviewPanel {
    /// White's, then Black's.
    accuracy: [f64; 2],
    /// The rating each side played like; `None` with too few moves to go on.
    ratings: [Option<u32>; 2],
    /// Each class with White's count and Black's.
    counts: Vec<(MoveClass, usize, usize)>,
    shown: Option<ShownVerdict>,
    graph: EvalGraph,
    /// The opening the game had reached at the position shown, said above the graph.
    opening: Option<Opening>,
}

impl ReviewPanel {
    pub fn new(
        (accuracy, ratings): ([f64; 2], [Option<u32>; 2]),
        counts: Vec<(MoveClass, usize, usize)>,
        (shown, graph): (Option<ShownVerdict>, EvalGraph),
    ) -> Self {
        Self {
            accuracy,
            ratings,
            counts,
            shown,
            graph,
            opening: None,
        }
    }

    pub fn opening(mut self, opening: Option<Opening>) -> Self {
        self.opening = opening;
        self
    }
}

impl RenderOnce for ReviewPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let pieces = BoardStyle::current(cx).pieces;
        let pawn = |side| piece_image(Piece::new(side, PieceKind::Pawn), pieces).path;
        let [white_rating, black_rating] = self.ratings.map(estimate);
        v_flex()
            .w_full()
            .gap_2()
            .p_3()
            .rounded(px(8.))
            .bg(theme.muted)
            .text_sm()
            .when_some(self.opening, |this, opening| {
                this.child(opening_bubble(&opening))
            })
            .child(self.graph)
            .when_some(self.shown, |this, shown| this.child(verdict(shown, muted)))
            .child(table_row(
                div(),
                [
                    player(pawn(Side::White), side_name(Side::White)),
                    player(pawn(Side::Black), side_name(Side::Black)),
                ],
            ))
            .child(table_row(
                row_name(tr!("analysis.review.accuracy")),
                [
                    chip(format!("{:.1}", self.accuracy[0]), Side::White),
                    chip(format!("{:.1}", self.accuracy[1]), Side::Black),
                ],
            ))
            .child(rule(cx))
            .children(
                self.counts
                    .into_iter()
                    .map(|(class, white, black)| count_row(class, white, black)),
            )
            .child(rule(cx))
            .child(table_row(
                row_name(tr!("analysis.review.game_rating")),
                [
                    chip(white_rating, Side::White),
                    chip(black_rating, Side::Black),
                ],
            ))
    }
}

/// The coach, Ferris, saying the opening in a white speech bubble, in light mode and dark.
fn opening_bubble(opening: &Opening) -> Div {
    h_flex()
        .w_full()
        .gap_3()
        .items_center()
        .child(
            img(FERRIS_PICTURE)
                .w(COACH_WIDTH)
                .h(COACH_HEIGHT)
                .flex_none(),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .px_4()
                .py_3()
                .rounded(px(12.))
                .bg(white())
                .text_color(rgb(BUBBLE_TEXT))
                .text_base()
                .font_weight(FontWeight::MEDIUM)
                .child(tr!("analysis.review.opening", name = opening.name.clone())),
        )
}

/// Ferris's picture is half again as wide as it is tall.
const COACH_WIDTH: Pixels = px(60.);
const COACH_HEIGHT: Pixels = px(40.);
/// The bubble's words: the darkest grey of the app's own, on white.
const BUBBLE_TEXT: u32 = 0x0011_1111;

/// `Nf3 is a Blunder`, with what was best below.
fn verdict(shown: ShownVerdict, muted: Hsla) -> Div {
    let line = tr!(
        "analysis.review.verdict",
        san = shown.san,
        verdict = shown.class.verdict()
    );
    v_flex()
        .gap_1()
        .py_1()
        .child(
            h_flex()
                .gap_2()
                .font_weight(FontWeight::SEMIBOLD)
                .child(img(shown.class.icon_path()).size_5())
                .child(line),
        )
        .when_some(shown.best, |this, best| {
            this.child(
                div()
                    .text_color(muted)
                    .child(tr!("analysis.review.best_was", best = best)),
            )
        })
}

/// A row: its name, then White's cell, a middle cell, and Black's, each in its own column.
fn table_row(name: Div, [white, black]: [Div; 2]) -> Div {
    table_row_with_middle(name, [white, black], div())
}

fn table_row_with_middle(name: Div, [white, black]: [Div; 2], middle: Div) -> Div {
    h_flex()
        .w_full()
        .gap_1()
        .py_1()
        .child(name.flex_1().min_w_0())
        .child(cell(white))
        .child(div().w(px(28.)).flex().justify_center().child(middle))
        .child(cell(black))
}

fn cell(content: Div) -> Div {
    div()
        .w(px(64.))
        .flex_shrink_0()
        .flex()
        .justify_center()
        .child(content)
}

fn row_name(name: impl Into<SharedString>) -> Div {
    div().font_weight(FontWeight::BOLD).child(name.into())
}

/// A side's picture and name over its column.
fn player(picture: &'static str, name: impl Into<SharedString>) -> Div {
    v_flex()
        .items_center()
        .gap_1()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .child(name.into()),
        )
        .child(
            div()
                .size_10()
                .rounded(px(6.))
                .bg(rgb(0x0080_8080).opacity(0.3))
                .flex()
                .items_center()
                .justify_center()
                .child(img(picture).size_8()),
        )
}

/// A number on a chip in the side's colours, as chess.com shows accuracy and rating.
fn chip(text: String, side: Side) -> Div {
    side_chip(text, side)
        .w_full()
        .py_1()
        .font_weight(FontWeight::EXTRA_BOLD)
}

fn rule(cx: &App) -> Div {
    div().h_px().my_1().bg(cx.theme().border)
}

/// The rating a side played like, as an estimate, or a dash with too few moves.
fn estimate(rating: Option<u32>) -> String {
    rating.map_or_else(|| "—".to_owned(), |rating| rating.to_string())
}

/// A kind of move: its name, White's count and Black's in its colour, and its icon between.
fn count_row(class: MoveClass, white: usize, black: usize) -> Div {
    let colour = class_colour(class);
    let count = |count: usize| {
        div()
            .font_weight(FontWeight::EXTRA_BOLD)
            .text_color(colour)
            .child(count.to_string())
    };
    table_row_with_middle(
        row_name(class.label()),
        [count(white), count(black)],
        div().child(img(class.icon_path()).size_5()),
    )
}

/// Game Review's star: the best move's icon, on Game Review's buttons and the review card.
pub const REVIEW_PICTURE: &str = MoveClass::Best.icon_path();

/// Each kind's colour, as chess.com paints it.
pub(super) fn class_colour(class: MoveClass) -> Hsla {
    let hex = match class {
        MoveClass::Brilliant => 0x001b_aca6,
        MoveClass::Great => 0x005c_8bb0,
        MoveClass::Best | MoveClass::Excellent => 0x0096_bc4b,
        MoveClass::Good => 0x0096_af8b,
        MoveClass::Book => 0x00a8_8865,
        MoveClass::Forced => 0x0097_9797,
        MoveClass::Inaccuracy => 0x00f7_c045,
        MoveClass::Mistake => 0x00e5_8f2a,
        MoveClass::Miss => 0x00ee_6b55,
        MoveClass::Blunder => 0x00ca_3431,
    };
    rgb(hex).into()
}
