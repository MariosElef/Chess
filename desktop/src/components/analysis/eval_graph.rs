use std::{cell::Cell, rc::Rc};

use gpui_kit::*;

use super::{review_panel::class_colour, side_chip::DARK_SIDE};
use crate::{
    enums::analysis::MoveClass,
    services::analysis::{graph_points, position_at},
};

type OnSelect = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// The game's evaluation over every move, as chess.com draws it: White's share in white over a
/// dark ground, a line through the middle, and the move on the board marked. A click goes to
/// the move under it.
#[derive(IntoElement)]
pub struct EvalGraph {
    shares: Vec<f64>,
    /// The position on the board: 0 is the start.
    viewing: usize,
    on_select: OnSelect,
}

/// The graph's height, and its colours: chess.com's dark ground, and the marker in a mistake's
/// orange.
const HEIGHT: Pixels = px(80.);
const GROUND: u32 = DARK_SIDE;
const MIDDLE: u32 = 0x0080_8080;

impl EvalGraph {
    pub fn new(
        shares: Vec<f64>,
        viewing: usize,
        on_select: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            shares,
            viewing,
            on_select: Rc::new(on_select),
        }
    }
}

impl RenderOnce for EvalGraph {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let points = graph_points(&self.shares);
        let count = self.shares.len();
        let viewing = self.viewing.min(count.saturating_sub(1));
        let marker = points.get(viewing).map(|(across, _)| *across);
        let drawn: Rc<Cell<Bounds<Pixels>>> = Rc::new(Cell::new(Bounds::default()));
        let (painted, clicked) = (Rc::clone(&drawn), drawn);
        let on_select = self.on_select;
        div()
            .id("eval-graph")
            .w_full()
            .h(HEIGHT)
            .flex_shrink_0()
            .rounded(px(6.))
            .overflow_hidden()
            .bg(rgb(GROUND))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                let bounds = clicked.get();
                let across = (event.position.x - bounds.origin.x) / bounds.size.width;
                on_select(position_at(across, count), window, cx);
            })
            .child(
                canvas(
                    move |bounds, _, _| painted.set(bounds),
                    move |bounds, (), window, _| {
                        paint_area(&points, bounds, window);
                        paint_line(bounds, 0.5, rgb(MIDDLE).into(), window);
                        if let Some(across) = marker {
                            paint_marker(bounds, across, window);
                        }
                    },
                )
                .size_full(),
            )
    }
}

/// White's area: from the bottom left, along the points, down to the bottom right.
fn paint_area(points: &[(f32, f32)], bounds: Bounds<Pixels>, window: &mut Window) {
    let at = |(across, down): (f32, f32)| {
        point(
            bounds.origin.x + bounds.size.width * across,
            bounds.origin.y + bounds.size.height * down,
        )
    };
    let mut path = PathBuilder::fill();
    path.move_to(at((0., 1.)));
    for &spot in points {
        path.line_to(at(spot));
    }
    path.line_to(at((1., 1.)));
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, white());
    }
}

/// A faint line across, `down` of the way from the top.
fn paint_line(bounds: Bounds<Pixels>, down: f32, colour: Hsla, window: &mut Window) {
    let top = bounds.origin.y + bounds.size.height * down;
    window.paint_quad(fill(
        Bounds::new(point(bounds.origin.x, top), size(bounds.size.width, px(1.))),
        colour.opacity(0.6),
    ));
}

/// The move on the board: an orange line from top to bottom.
fn paint_marker(bounds: Bounds<Pixels>, across: f32, window: &mut Window) {
    let left = bounds.origin.x + bounds.size.width * across - px(1.);
    window.paint_quad(fill(
        Bounds::new(
            point(left, bounds.origin.y),
            size(px(2.), bounds.size.height),
        ),
        class_colour(MoveClass::Mistake),
    ));
}
