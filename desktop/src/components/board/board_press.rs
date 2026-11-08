//! A press on a piece: before a drag begins, the piece glides from its square until its
//! centre is on the cursor, drawn by the board above every square.
#![expect(
    clippy::float_arithmetic,
    reason = "the pressed piece sits a share of the board from its edge"
)]

use std::time::Duration;

use chess_domain::Square;
use gpui_kit::*;

use crate::services::board::Cell;

/// How long a pressed piece takes to glide from where it stood to the cursor's centre.
const CENTRE_GLIDE: Duration = Duration::from_millis(110);

/// The board's press, if any: the square pressed, how far the cursor is from its centre, and
/// which press this is, so each one glides afresh.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BoardPress {
    pressed: Option<(Square, Point<Pixels>)>,
    run: usize,
}

impl BoardPress {
    pub fn pressed(&self) -> Option<(Square, Point<Pixels>)> {
        self.pressed
    }

    pub fn run(&self) -> usize {
        self.run
    }
}

/// The board's press, kept across redraws.
pub fn press_state(window: &mut Window, cx: &mut App) -> Entity<BoardPress> {
    window.use_keyed_state("board-press", cx, |_, _| BoardPress::default())
}

/// A piece pressed on `square`, with the cursor `shift` from the square's centre.
pub fn press_down(
    state: &Entity<BoardPress>,
    (square, shift): (Square, Point<Pixels>),
    window: &mut Window,
    cx: &mut App,
) {
    state.update(cx, |press, _| {
        press.pressed = Some((square, shift));
        press.run += 1;
    });
    window.refresh();
}

/// Let go anywhere: the piece is back on its square, unless a drag took it.
pub fn press_up(state: &Entity<BoardPress>, window: &mut Window, cx: &mut App) {
    if state.read(cx).pressed.is_none() {
        return;
    }
    state.update(cx, |press, _| press.pressed = None);
    window.refresh();
}

/// The pressed piece on its cell, gliding by `shift` until its centre is on the cursor; `inset`
/// keeps it the size it is on its square.
pub fn pressed_layer(
    picture: &'static str,
    (cell, shift): (Cell, Point<Pixels>),
    (run, inset): (usize, Pixels),
) -> Div {
    let id = ElementId::NamedInteger("press".into(), run as u64);
    div()
        .absolute()
        .left(relative(f32::from(cell.0) / 8.))
        .top(relative(f32::from(cell.1) / 8.))
        .size(relative(1. / 8.))
        .child(
            div()
                .relative()
                .size_full()
                .p(inset)
                .child(
                    img(picture)
                        .size_full()
                        .debug_selector(|| "pressed-piece".to_owned()),
                )
                .with_animation(
                    id,
                    Animation::new(CENTRE_GLIDE).with_easing(ease_out_quint()),
                    move |this, progress| this.left(shift.x * progress).top(shift.y * progress),
                ),
        )
}
