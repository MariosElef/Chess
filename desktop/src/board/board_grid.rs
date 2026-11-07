use chess_domain::{Board, Square};
use gpui_kit::{
    component::{ActiveTheme as _, h_flex, v_flex},
    *,
};

use super::{board_square::BoardSquare, config::BoardColors};

/// The 8×8 board seen from White's side, square at whatever height its parent gives it.
#[derive(IntoElement)]
pub struct BoardGrid {
    board: Board,
}

impl BoardGrid {
    pub fn new(board: Board) -> Self {
        Self { board }
    }
}

impl RenderOnce for BoardGrid {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = BoardColors::current(cx);
        let rows = (0..8).rev().map(|rank| {
            h_flex()
                .flex_1()
                .items_stretch()
                .children((0..8).map(|file| {
                    let square = Square::new(file, rank).expect("0..8 stays on the board");
                    BoardSquare::new(square, self.board.piece_at(square), colors)
                }))
        });

        v_flex()
            .h_full()
            .max_w_full()
            .aspect_square()
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .children(rows)
    }
}
