use std::rc::Rc;

use chess_domain::{Board, Square};
use gpui_kit::{
    component::{ActiveTheme as _, h_flex, v_flex},
    *,
};

use super::{
    board_square::{BoardSquare, OnSquareClick},
    config::BoardColors,
};

/// The 8×8 board seen from White's side, square at whatever height its parent gives it.
#[derive(IntoElement)]
pub struct BoardGrid {
    board: Board,
    selected: Option<Square>,
    targets: Vec<Square>,
    on_square_click: Option<OnSquareClick>,
}

impl BoardGrid {
    pub fn new(board: Board) -> Self {
        Self {
            board,
            selected: None,
            targets: Vec::new(),
            on_square_click: None,
        }
    }

    /// The square whose piece is picked up, drawn highlighted.
    pub fn selected(mut self, selected: Option<Square>) -> Self {
        self.selected = selected;
        self
    }

    /// The squares that piece can move to, each marked with a dot (or a ring around a capture).
    pub fn targets(mut self, targets: Vec<Square>) -> Self {
        self.targets = targets;
        self
    }

    pub fn on_square_click(
        mut self,
        handler: impl Fn(Square, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_square_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for BoardGrid {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = BoardColors::current(cx);
        v_flex()
            .h_full()
            .max_w_full()
            .aspect_square()
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .children((0..8).rev().map(|rank| self.rank_row(rank, colors)))
    }
}

impl BoardGrid {
    /// One rank, a to h; ranks are drawn 8 down to 1 so White sits at the bottom.
    fn rank_row(&self, rank: u8, colors: BoardColors) -> Div {
        h_flex()
            .flex_1()
            .items_stretch()
            .children((0..8).map(|file| self.square(file, rank, colors)))
    }

    fn square(&self, file: u8, rank: u8, colors: BoardColors) -> BoardSquare {
        let square = Square::new(file, rank).expect("0..8 stays on the board");
        BoardSquare::new(square, self.board.piece_at(square), colors)
            .selected(self.selected == Some(square))
            .target(self.targets.contains(&square))
            .on_click(self.on_square_click.clone())
    }
}
