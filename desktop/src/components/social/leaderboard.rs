use gpui_kit::{
    component::{ActiveTheme as _, h_flex, v_flex},
    prelude::FluentBuilder as _,
    *,
};

use super::avatar::pawn_avatar;
use crate::{
    components::{play::speed_picture, shell::PUZZLES_PICTURE},
    enums::social::LeaderBoard,
    services::social::{LeaderRow, PODIUM},
};

/// The friends' leaderboard per speed and puzzles: the top three, and the player's own place.
#[derive(IntoElement)]
pub struct Leaderboard {
    boards: Vec<(LeaderBoard, Vec<LeaderRow>)>,
}

impl Leaderboard {
    pub fn new(boards: Vec<(LeaderBoard, Vec<LeaderRow>)>) -> Self {
        Self { boards }
    }
}

impl RenderOnce for Leaderboard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let sections: Vec<Div> = self
            .boards
            .into_iter()
            .map(|(board, rows)| board_section(board, rows, cx))
            .collect();
        v_flex()
            .w(px(340.))
            .flex_shrink_0()
            .rounded(px(8.))
            .bg(cx.theme().muted)
            .overflow_hidden()
            .child(
                div()
                    .px_4()
                    .py_3()
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .child("Friends Leaderboard"),
            )
            .children(sections)
    }
}

/// One board: its band with its picture and name, then its places.
fn board_section(board: LeaderBoard, rows: Vec<LeaderRow>, cx: &App) -> Div {
    let theme = cx.theme();
    let band = h_flex()
        .gap_2()
        .px_4()
        .py_2()
        .bg(theme.secondary)
        .font_weight(FontWeight::SEMIBOLD)
        .when_some(board_picture(board), |this, path| {
            this.child(img(path).size_5())
        })
        .child(board.label());
    let empty = rows.is_empty();
    let places: Vec<Div> = rows.into_iter().map(|row| place_row(row, cx)).collect();
    v_flex().child(band).children(places).when(empty, |this| {
        this.child(
            div()
                .px_4()
                .py_2()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child("No ratings yet"),
        )
    })
}

/// "#1", a small picture, the name, and the rating at the right; the player's own in bold.
fn place_row(row: LeaderRow, cx: &App) -> Div {
    let below = row.place > PODIUM;
    let muted = cx.theme().muted_foreground;
    h_flex()
        .gap_3()
        .px_4()
        .py_1p5()
        .text_sm()
        .when(row.me, |this| this.font_weight(FontWeight::BOLD))
        .when(below, |this| {
            this.border_t_1().border_color(cx.theme().border)
        })
        .child(
            div()
                .w(px(24.))
                .text_color(muted)
                .child(format!("#{}", row.place)),
        )
        .child(pawn_avatar(px(24.)))
        .child(div().flex_1().min_w_0().child(row.name))
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .child(row.rating.to_string()),
        )
}

fn board_picture(board: LeaderBoard) -> Option<&'static str> {
    match board {
        LeaderBoard::Speed(category) => speed_picture(category),
        LeaderBoard::Puzzles => Some(PUZZLES_PICTURE),
    }
}
