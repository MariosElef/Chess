//! Arrows drawn over the board, from one square's centre toward another's.
#![expect(
    clippy::float_arithmetic,
    reason = "the arrow is geometry in screen space"
)]

use gpui_kit::*;

use crate::services::board::{ArrowShape, Cell, arrow_shape, bend_corner, centre};

/// Shaft width, as a fraction of a square.
const SHAFT_WIDTH: f32 = 0.18;

/// Paints every arrow over the board's bounds; one square is an eighth of the board.
pub fn paint_arrows(
    bounds: Bounds<Pixels>,
    arrows: &[(Cell, Cell)],
    color: Hsla,
    window: &mut Window,
) {
    let cell = bounds.size.width.as_f32() / 8.0;
    let origin = (bounds.origin.x.as_f32(), bounds.origin.y.as_f32());
    for &(from, to) in arrows {
        let (start, end) = (centre(from, cell, origin), centre(to, cell, origin));
        let shape = arrow_shape(start, end, bend_corner(from, to, start, end), cell);
        paint_shape(&shape, cell, color, window);
    }
}

/// One square's width: the board is eight squares across.
pub fn square_width(bounds: Bounds<Pixels>) -> Pixels {
    bounds.size.width / 8.0
}

fn paint_shape(shape: &ArrowShape, cell: f32, color: Hsla, window: &mut Window) {
    let mut shaft = PathBuilder::stroke(px(SHAFT_WIDTH * cell));
    if let Some((&first, rest)) = shape.shaft.split_first() {
        shaft.move_to(point_at(first));
        for &next in rest {
            shaft.line_to(point_at(next));
        }
    }
    let mut head = PathBuilder::fill();
    head.move_to(point_at(shape.head[0]));
    head.line_to(point_at(shape.head[1]));
    head.line_to(point_at(shape.head[2]));
    head.close();
    for path in [shaft.build(), head.build()].into_iter().flatten() {
        window.paint_path(path, color);
    }
}

fn point_at((x, y): (f32, f32)) -> Point<Pixels> {
    point(px(x), px(y))
}
