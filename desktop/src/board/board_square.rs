use std::rc::Rc;

use chess_domain::{Piece, Square};
use gpui_kit::{prelude::FluentBuilder as _, *};

use super::config::{BoardColors, piece_image};

pub type OnSquareClick = Rc<dyn Fn(Square, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct BoardSquare {
    square: Square,
    piece: Option<Piece>,
    colors: BoardColors,
    selected: bool,
    target: bool,
    on_click: Option<OnSquareClick>,
}

impl BoardSquare {
    pub fn new(square: Square, piece: Option<Piece>, colors: BoardColors) -> Self {
        Self {
            square,
            piece,
            colors,
            selected: false,
            target: false,
            on_click: None,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn target(mut self, target: bool) -> Self {
        self.target = target;
        self
    }

    pub fn on_click(mut self, handler: Option<OnSquareClick>) -> Self {
        self.on_click = handler;
        self
    }
}

impl RenderOnce for BoardSquare {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let square = self.square;
        div()
            .id(("square", square.index()))
            .relative()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .bg(self.colors.background(square, self.selected))
            .when_some(self.piece, |this, piece| this.child(piece_picture(piece)))
            .when(self.target, |this| {
                this.child(move_marker(self.piece.is_some(), self.colors))
            })
            .when_some(self.on_click, |this, handler| {
                this.on_click(move |_, window, cx| handler(square, window, cx))
            })
    }
}

fn piece_picture(piece: Piece) -> Img {
    img(piece_image(piece).path).size_full()
}

/// A dot on an empty square; a ring around a piece that can be captured, so the piece stays visible.
fn move_marker(is_capture: bool, colors: BoardColors) -> Div {
    if is_capture {
        capture_ring(colors)
    } else {
        move_dot(colors)
    }
}

fn move_dot(colors: BoardColors) -> Div {
    div()
        .size(relative(0.3))
        .rounded_full()
        .bg(colors.move_hint)
}

fn capture_ring(colors: BoardColors) -> Div {
    div()
        .absolute()
        .inset_0()
        .rounded_full()
        .border_4()
        .border_color(colors.move_hint)
}
