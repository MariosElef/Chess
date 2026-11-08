use std::{cell::Cell, rc::Rc};

use chess_domain::{Piece, Square};
use gpui_kit::{prelude::FluentBuilder as _, *};

use super::{board_colors::BoardColors, piece_images::piece_image};
use crate::enums::board::SquareHighlight;

pub type OnSquareClick = Rc<dyn Fn(Square, &mut Window, &mut App)>;
pub type OnPieceDrop = Rc<dyn Fn(Square, Square, &mut Window, &mut App)>;

/// The right button pressed and released over squares, for marks and arrows.
#[derive(Clone)]
pub struct RightButton {
    pub on_down: OnSquareClick,
    pub on_up: OnSquareClick,
}

/// One square's width on screen, measured each time the board is drawn.
pub type SquareSize = Rc<Cell<Pixels>>;

/// The value carried while a piece is dragged across the board.
#[derive(Debug, Clone)]
pub struct DraggedPiece {
    pub from: Square,
    pub piece: Piece,
    /// So the dragged piece stays the size it is on the board.
    pub size: SquareSize,
}

#[derive(IntoElement)]
pub struct BoardSquare {
    square: Square,
    piece: Option<Piece>,
    colors: BoardColors,
    highlight: SquareHighlight,
    target: bool,
    draggable: bool,
    size: SquareSize,
    /// The file letter on the bottom edge, the rank number on the left edge.
    file_label: Option<char>,
    rank_label: Option<char>,
    on_click: Option<OnSquareClick>,
    on_drop: Option<OnPieceDrop>,
    right_button: Option<RightButton>,
}

impl BoardSquare {
    pub fn new(
        square: Square,
        piece: Option<Piece>,
        colors: BoardColors,
        size: SquareSize,
    ) -> Self {
        Self {
            square,
            piece,
            colors,
            highlight: SquareHighlight::None,
            target: false,
            draggable: false,
            size,
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let square = self.square;
        let coordinate = self.colors.coordinate(square);
        let lifted = lifted_state(square, window, cx);
        let shown_piece = self.piece.filter(|_| !is_lifted(&lifted, cx));
        div()
            .id(("square", square.index()))
            .debug_selector(|| format!("square-{square}"))
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
            .when_some(shown_piece, |this, piece| {
                this.child(piece_picture(piece, square))
            })
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
                        size: self.size,
                    },
                    move |dragged, grab_offset, window, cx| {
                        lifted.update(cx, |lifted, _| *lifted = true);
                        drag_preview(dragged, grab_offset, window, cx)
                    },
                )
            })
    }
}

/// Whether this square's piece is the one being dragged, kept across redraws so the square can
/// leave its piece out while it follows the cursor.
fn lifted_state(square: Square, window: &mut Window, cx: &mut App) -> Entity<bool> {
    window.use_keyed_state(("lifted", square.index()), cx, |_, _| false)
}

/// A piece is lifted only while a drag is going on; once it ends, the flag is cleared so the
/// piece shows again (or has moved, if it was dropped on a legal square).
fn is_lifted(lifted: &Entity<bool>, cx: &mut App) -> bool {
    if cx.has_active_drag() {
        return *lifted.read(cx);
    }
    if *lifted.read(cx) {
        lifted.update(cx, |lifted, _| *lifted = false);
    }
    false
}

fn piece_picture(piece: Piece, square: Square) -> Img {
    img(piece_image(piece).path)
        .size_full()
        .debug_selector(|| format!("piece-{square}"))
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
    grab_offset: Point<Pixels>,
    _: &mut Window,
    cx: &mut App,
) -> Entity<DragPreview> {
    let (piece, size) = (dragged.piece, dragged.size.get());
    let shift = centre_on_cursor(grab_offset, size);
    cx.new(|_| DragPreview { piece, size, shift })
}

/// GPUI draws the dragged view where the square was grabbed; moving it by the grab point minus
/// half a square puts the piece's centre on the cursor, wherever it was picked up.
fn centre_on_cursor(grab_offset: Point<Pixels>, size: Pixels) -> Point<Pixels> {
    let half = size / 2.;
    point(grab_offset.x - half, grab_offset.y - half)
}

/// The piece that follows the pointer while it is dragged, as big as on the board and centred
/// on the cursor.
pub struct DragPreview {
    piece: Piece,
    size: Pixels,
    shift: Point<Pixels>,
}

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .left(self.shift.x)
            .top(self.shift.y)
            .size(self.size)
            .child(img(piece_image(self.piece).path).size_full())
    }
}
