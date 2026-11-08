use std::rc::Rc;

use chess_domain::{Piece, Square};
use gpui_kit::{prelude::FluentBuilder as _, *};

use super::{board_colors::BoardColors, piece_images::piece_image};
use crate::enums::{board::SquareHighlight, settings::PieceSet};

pub type OnSquareClick = Rc<dyn Fn(Square, &mut Window, &mut App)>;
/// A piece pressed on its square, with where the cursor is on the window.
pub type OnPieceGrab = Rc<dyn Fn(Square, Point<Pixels>, &mut Window, &mut App)>;

/// Moved this far from where it was pressed, a held piece is being dragged, and a press and
/// release this far apart was a drag, not a click.
pub const DRAG_START: Pixels = px(4.);

/// The right button pressed and released over squares, for marks and arrows.
#[derive(Clone)]
pub struct RightButton {
    pub on_down: OnSquareClick,
    pub on_up: OnSquareClick,
}

#[derive(IntoElement)]
pub struct BoardSquare {
    square: Square,
    piece: Option<Piece>,
    pieces: PieceSet,
    colors: BoardColors,
    highlight: SquareHighlight,
    target: bool,
    draggable: bool,
    /// The file letter on the bottom edge, the rank number on the left edge.
    file_label: Option<char>,
    rank_label: Option<char>,
    /// An icon in the top-right corner, by asset path: a reviewed move's class.
    badge: Option<&'static str>,
    /// The piece is elsewhere for now: held by the player, or sliding onto the square.
    hide_piece: bool,
    on_click: Option<OnSquareClick>,
    on_grab: Option<OnPieceGrab>,
    right_button: Option<RightButton>,
}

impl BoardSquare {
    pub fn new(
        square: Square,
        (piece, pieces): (Option<Piece>, PieceSet),
        colors: BoardColors,
    ) -> Self {
        Self {
            square,
            piece,
            pieces,
            colors,
            highlight: SquareHighlight::None,
            target: false,
            draggable: false,
            file_label: None,
            rank_label: None,
            badge: None,
            hide_piece: false,
            on_click: None,
            on_grab: None,
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

    /// A press on the square's piece picks it up, when it can be moved.
    pub fn on_grab(mut self, handler: Option<OnPieceGrab>) -> Self {
        self.on_grab = handler;
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
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let square = self.square;
        let coordinate = self.colors.coordinate(square);
        let grabbable = self.draggable && self.piece.is_some();
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
            .when_some(self.piece.filter(|_| !self.hide_piece), |this, piece| {
                this.child(piece_picture(piece, square, self.pieces))
            })
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
                this.on_click(move |event, window, cx| {
                    // A piece dragged away and back lands where it started: that was no click.
                    if !was_dragged(event) {
                        handler(square, window, cx);
                    }
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
            .when_some(self.on_grab.filter(|_| grabbable), |this, on_grab| {
                this.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    on_grab(square, event.position, window, cx);
                })
            })
    }
}

/// Whether the button went down and up far enough apart to have been a drag.
fn was_dragged(event: &ClickEvent) -> bool {
    match event {
        ClickEvent::Mouse(click) => {
            let moved = click.up.position - click.down.position;
            moved.x.abs() >= DRAG_START || moved.y.abs() >= DRAG_START
        }
        ClickEvent::Keyboard(_) | ClickEvent::Touch(_) => false,
    }
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
