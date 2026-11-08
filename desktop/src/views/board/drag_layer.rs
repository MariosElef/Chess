//! The piece the player holds, drawn over the board and following the cursor. Only this view
//! redraws as the mouse moves: the board under it is left as it was.

use chess_domain::Square;
use gpui_kit::*;

use crate::{
    components::board::{BoardBounds, DRAG_START, HeldPiece},
    services::board::{cell_of, square_in_cell},
    views::board::BoardView,
};

/// A piece held down: where from, its picture, which way up the board is, and the cursor.
struct Held {
    from: Square,
    picture: &'static str,
    flipped: bool,
    pressed_at: Point<Pixels>,
    cursor: Point<Pixels>,
    dragging: bool,
}

pub struct DragLayer {
    held: Option<Held>,
    bounds: BoardBounds,
    board: WeakEntity<BoardView>,
    /// Which hold this is, so each one glides afresh.
    run: usize,
}

impl DragLayer {
    pub fn new(bounds: BoardBounds, board: WeakEntity<BoardView>) -> Self {
        Self {
            held: None,
            bounds,
            board,
            run: 0,
        }
    }

    /// The piece on `from` is pressed with the cursor at `cursor`, on the window.
    pub fn hold(
        &mut self,
        (from, picture): (Square, &'static str),
        flipped: bool,
        cursor: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        self.held = Some(Held {
            from,
            picture,
            flipped,
            pressed_at: cursor,
            cursor,
            dragging: false,
        });
        self.run += 1;
        cx.notify();
    }

    /// Lets go without a move: the piece goes back to its square.
    pub fn let_go(&mut self, cx: &mut Context<Self>) {
        if self.held.take().is_some() {
            cx.notify();
        }
    }

    fn moved(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(held) = self.held.as_mut() else {
            return;
        };
        held.cursor = position;
        let moved = position - held.pressed_at;
        held.dragging |= moved.x.abs() >= DRAG_START || moved.y.abs() >= DRAG_START;
        cx.notify();
    }

    /// The button came up: the hold ends, and a drag lands on the square under the cursor.
    fn released(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) -> Option<Landing> {
        let held = self.held.take()?;
        cx.notify();
        Some(Landing {
            from: held.from,
            to: self.square_under(position, held.flipped),
            dragged: held.dragging,
        })
    }

    /// The square drawn under `position`, on the window; `None` off the board.
    fn square_under(&self, position: Point<Pixels>, flipped: bool) -> Option<Square> {
        let bounds = self.bounds.get();
        if !bounds.contains(&position) {
            return None;
        }
        let width = bounds.size.width / 8.;
        let inside = position - bounds.origin;
        square_in_cell(
            (cell_index(inside.x / width), cell_index(inside.y / width)),
            flipped,
        )
    }
}

/// Where a released piece came from and went.
struct Landing {
    from: Square,
    to: Option<Square>,
    dragged: bool,
}

/// A column or row from a share of the board's width, 0 to 8.
fn cell_index(share: f32) -> u8 {
    // Checked to lie on the board, so it fits.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "0..8"
    )]
    let index = share.floor().clamp(0., 7.) as u8;
    index
}

impl Render for DragLayer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(held) = &self.held else {
            return div().into_any_element();
        };
        let bounds = self.bounds.get();
        let width = bounds.size.width / 8.;
        let corner = |square: Square| {
            let (column, row) = cell_of(square, held.flipped);
            point(width * f32::from(column), width * f32::from(row))
        };
        let from = corner(held.from) + point(width / 2., width / 2.);
        let over = held
            .dragging
            .then(|| self.square_under(held.cursor, held.flipped))
            .flatten()
            .map(corner);
        let layer = cx.entity().downgrade();
        div()
            .absolute()
            .inset_0()
            .child(HeldPiece::new(
                (held.picture, width),
                (from, held.cursor - bounds.origin),
                over,
                self.run,
            ))
            .child(follow_the_mouse(layer, self.board.clone()))
            .into_any_element()
    }
}

/// While a piece is held, every move and release on the window reaches the layer.
fn follow_the_mouse(layer: WeakEntity<DragLayer>, board: WeakEntity<BoardView>) -> Canvas<()> {
    canvas(
        |_, _, _| {},
        move |_, (), window, _| {
            let mover = layer.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                if phase == DispatchPhase::Capture {
                    let _ = mover.update(cx, |layer, cx| layer.moved(event.position, cx));
                }
            });
            let (releaser, board) = (layer.clone(), board.clone());
            window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                if phase != DispatchPhase::Capture || event.button != MouseButton::Left {
                    return;
                }
                let landing = releaser
                    .update(cx, |layer, cx| layer.released(event.position, cx))
                    .ok()
                    .flatten();
                if let Some(landing) = landing {
                    let _ = board.update(cx, |board, cx| {
                        board.land(landing.from, landing.to, landing.dragged, cx);
                    });
                }
            });
        },
    )
    .absolute()
    .inset_0()
}
