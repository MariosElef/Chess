//! Real mouse input for the page tests: clicks and drags on squares found by their
//! `square-e4` selectors.
use gpui_kit::{Modifiers, MouseButton, Pixels, Point, VisualTestContext, point, px};

/// Far enough to start a drag, so a press is not taken for a click.
const DRAG_NUDGE: Pixels = px(6.);

/// The middle of the element drawn with `selector`, once the window has settled.
pub fn centre_of(cx: &mut VisualTestContext, selector: &'static str) -> Point<Pixels> {
    cx.run_until_parked();
    cx.debug_bounds(selector)
        .expect("the square is drawn")
        .center()
}

pub fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let at = centre_of(cx, selector);
    cx.simulate_click(at, Modifiers::none());
}

/// Presses on `selector` and moves past the drag threshold, leaving the button down.
pub fn lift(cx: &mut VisualTestContext, selector: &'static str) -> Point<Pixels> {
    let start = centre_of(cx, selector);
    let left = MouseButton::Left;
    cx.simulate_mouse_down(start, left, Modifiers::none());
    cx.simulate_mouse_move(
        start + point(DRAG_NUDGE, DRAG_NUDGE),
        Some(left),
        Modifiers::none(),
    );
    cx.run_until_parked();
    start
}

/// Lets go of a lifted piece at `at`.
pub fn release(cx: &mut VisualTestContext, at: Point<Pixels>) {
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
}

/// Press on one square, move to the other, release.
pub fn drag(cx: &mut VisualTestContext, from: &'static str, to: &'static str) {
    lift(cx, from);
    let end = centre_of(cx, to);
    cx.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::none());
    release(cx, end);
}
