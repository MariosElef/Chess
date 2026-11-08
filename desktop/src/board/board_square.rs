use std::rc::Rc;

use chess_domain::{Piece, Square};
use gpui_kit::{prelude::FluentBuilder as _, *};

use super::config::{BoardColors, SquareHighlight, piece_image};

pub type OnSquareClick = Rc<dyn Fn(Square, &mut Window, &mut App)>;
pub type OnPieceDrop = Rc<dyn Fn(Square, Square, &mut Window, &mut App)>;

/// The right button pressed and released over squares, for marks and arrows.
#[derive(Clone)]
pub struct RightButton {
    pub on_down: OnSquareClick,
    pub on_up: OnSquareClick,
}

/// The value carried while a piece is dragged across the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DraggedPiece {
    pub from: Square,
    pub piece: Piece,
}

#[derive(IntoElement)]
pub struct BoardSquare {
    square: Square,
    piece: Option<Piece>,
    colors: BoardColors,
    highlight: SquareHighlight,
    target: bool,
    draggable: bool,
    /// The file letter on the bottom edge, the rank number on the left edge.
    file_label: Option<char>,
    rank_label: Option<char>,
    on_click: Option<OnSquareClick>,
    on_drop: Option<OnPieceDrop>,
    right_button: Option<RightButton>,
}

impl BoardSquare {
    pub fn new(square: Square, piece: Option<Piece>, colors: BoardColors) -> Self {
        Self {
            square,
            piece,
            colors,
            highlight: SquareHighlight::None,
            target: false,
            draggable: false,
            file_label: None,
            rank_label: None,
            on_click: None,
            on_drop: None,
            right_button: None,
        }
    }

    pub fn right_button(mut self, handlers: Option<RightButton>) -> Self {
        self.right_button = handlers;
        self
    }

    pub fn highlight(mut self, highlight: SquareHighlight) -> Self {
        self.highlight = highlight;
        self
    }

    pub fn target(mut self, target: bool) -> Self {
        self.target = target;
        self
    }

    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    pub fn labels(mut self, file_label: Option<char>, rank_label: Option<char>) -> Self {
        self.file_label = file_label;
        self.rank_label = rank_label;
        self
    }

    pub fn on_click(mut self, handler: Option<OnSquareClick>) -> Self {
        self.on_click = handler;
        self
    }

    pub fn on_drop(mut self, handler: Option<OnPieceDrop>) -> Self {
        self.on_drop = handler;
        self
    }
}

impl RenderOnce for BoardSquare {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let square = self.square;
        let coordinate = self.colors.coordinate(square);
        div()
            .id(("square", square.index()))
            .relative()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .bg(self.colors.background(square, self.highlight))
            .when_some(self.rank_label, |this, label| {
                this.child(rank_label(label, coordinate))
            })
            .when_some(self.file_label, |this, label| {
                this.child(file_label(label, coordinate))
            })
            .when_some(self.piece, |this, piece| this.child(piece_picture(piece)))
            .when(self.target, |this| {
                this.child(move_marker(self.piece.is_some(), self.colors))
            })
            .when_some(self.on_click, |this, handler| {
                this.on_click(move |_, window, cx| handler(square, window, cx))
            })
            .when_some(self.on_drop, |this, handler| {
                this.on_drop(move |dragged: &DraggedPiece, window, cx| {
                    handler(dragged.from, square, window, cx);
                })
            })
            .when_some(self.right_button, |this, handlers| {
                let RightButton { on_down, on_up } = handlers;
                this.on_mouse_down(MouseButton::Right, move |_, window, cx| {
                    on_down(square, window, cx);
                })
                .on_mouse_up(MouseButton::Right, move |_, window, cx| {
                    on_up(square, window, cx);
                })
            })
            .when_some(self.piece.filter(|_| self.draggable), |this, piece| {
                this.on_drag(
                    DraggedPiece {
                        from: square,
                        piece,
                    },
                    drag_preview,
                )
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

fn rank_label(label: char, color: Hsla) -> Div {
    div()
        .absolute()
        .top_0p5()
        .left_1()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color)
        .child(label.to_string())
}

fn file_label(label: char, color: Hsla) -> Div {
    div()
        .absolute()
        .bottom_0p5()
        .right_1()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color)
        .child(label.to_string())
}

fn drag_preview(
    dragged: &DraggedPiece,
    _: Point<Pixels>,
    _: &mut Window,
    cx: &mut App,
) -> Entity<DragPreview> {
    let piece = dragged.piece;
    cx.new(|_| DragPreview { piece })
}

/// The piece that follows the pointer while it is dragged.
pub struct DragPreview {
    piece: Piece,
}

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        img(piece_image(self.piece).path).size(rems(4.))
    }
}
