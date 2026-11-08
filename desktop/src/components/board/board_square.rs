use std::{cell::Cell, rc::Rc};

use chess_domain::{Piece, Square};
use gpui_kit::{prelude::FluentBuilder as _, *};

use super::{
    board_colors::BoardColors,
    board_press::{BoardPress, press_down, press_up},
    board_style::BoardStyle,
    piece_images::piece_image,
};
use crate::enums::{board::SquareHighlight, settings::PieceSet};

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
    /// An icon in the top-right corner, by asset path: a reviewed move's class.
    badge: Option<&'static str>,
    /// A piece is sliding onto the square: it shows when it arrives.
    hide_piece: bool,
    /// The board's press: a press on this square's piece glides it to the cursor.
    press: Option<Entity<BoardPress>>,
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
            badge: None,
            hide_piece: false,
            press: None,
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

    pub fn press(mut self, press: Entity<BoardPress>) -> Self {
        self.press = Some(press);
        self
    }

    pub fn hide_piece(mut self, hide: bool) -> Self {
        self.hide_piece = hide;
        self
    }

    pub fn badge(mut self, badge: Option<&'static str>) -> Self {
        self.badge = badge;
        self
    }
}

impl RenderOnce for BoardSquare {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let square = self.square;
        let coordinate = self.colors.coordinate(square);
        let lifted = lifted_state(square, window, cx);
        let hidden = self.hide_piece || is_lifted(&lifted, cx);
        let pieces = BoardStyle::current(cx).pieces;
        let bounds = Rc::new(Cell::new(Bounds::default()));
        let grabbable = self.draggable && self.piece.is_some();
        let press = self.press.clone();
        let right_press = press.clone();
        let pressed_here = press.as_ref().is_some_and(|state| {
            state
                .read(cx)
                .pressed()
                .is_some_and(|(pressed, _)| pressed == square)
        });
        div()
            .id(("square", square.index()))
            .debug_selector(|| format!("square-{square}"))
            .relative()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .bg(self.colors.background(square, self.highlight))
            // Always there, so the square keeps its size; white while a piece hovers over it.
            .border(HOVER_BORDER)
            .border_color(transparent_black())
            .drag_over::<DraggedPiece>(|style, _, _, _| style.border_color(hover_border_color()))
            .when_some(self.rank_label, |this, label| {
                this.child(rank_label(label, coordinate))
            })
            .when_some(self.file_label, |this, label| {
                this.child(file_label(label, coordinate))
            })
            .child(measure(Rc::clone(&bounds)))
            // A pressed piece is drawn by the board, gliding to the cursor above the squares.
            .when_some(
                self.piece.filter(|_| !hidden && !pressed_here),
                |this, piece| this.child(piece_picture(piece, square, pieces)),
            )
            .when_some(self.badge, |this, path| {
                this.child(
                    img(path)
                        .absolute()
                        .top_0p5()
                        .right_0p5()
                        .size(relative(0.36)),
                )
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
                let held = right_press;
                this.on_mouse_down(MouseButton::Right, move |_, window, cx| {
                    // A right-click while a piece is held or pressed puts it back, drawing nothing.
                    let dragged = cx.stop_active_drag(window);
                    let pressed = held
                        .as_ref()
                        .is_some_and(|press| press.read(cx).pressed().is_some());
                    if let Some(press) = &held {
                        press_up(press, window, cx);
                    }
                    if !dragged && !pressed {
                        on_down(square, window, cx);
                    }
                })
                .on_mouse_up(MouseButton::Right, move |_, window, cx| {
                    on_up(square, window, cx);
                })
            })
            .when_some(press.filter(|_| grabbable), |this, press| {
                let (down, up, up_out) = (press.clone(), press.clone(), press);
                this.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    let shift = event.position - bounds.get().center();
                    press_down(&down, (square, shift), window, cx);
                })
                .on_mouse_up(MouseButton::Left, move |_, window, cx| {
                    press_up(&up, window, cx);
                })
                .on_mouse_up_out(MouseButton::Left, move |_, window, cx| {
                    press_up(&up_out, window, cx);
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

/// Notes where the square is drawn, for a press to know how far the cursor is from its centre.
fn measure(bounds: Rc<Cell<Bounds<Pixels>>>) -> Canvas<()> {
    canvas(move |drawn, _, _| bounds.set(drawn), |_, _, _, _| {})
        .absolute()
        .inset_0()
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

fn piece_picture(piece: Piece, square: Square, pieces: PieceSet) -> Img {
    img(piece_image(piece, pieces).path)
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

/// The outline of the square a dragged piece hovers over.
pub const HOVER_BORDER: Pixels = px(3.);

fn hover_border_color() -> Hsla {
    hsla(0., 0., 1., 0.65)
}

/// The piece that follows the pointer while it is dragged, as big as on the board and centred
/// on the cursor: the press already glided it there.
pub struct DragPreview {
    piece: Piece,
    size: Pixels,
    shift: Point<Pixels>,
}

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .left(self.shift.x)
            .top(self.shift.y)
            .size(self.size)
            .child(img(piece_image(self.piece, BoardStyle::current(cx).pieces).path).size_full())
    }
}
