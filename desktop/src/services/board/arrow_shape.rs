//! The shape of an arrow on the board, from one square's centre toward another's, in screen
//! units: where its shaft runs, where it bends, and the corners of its head.
#![expect(
    clippy::float_arithmetic,
    reason = "the arrow is geometry in screen space"
)]

/// Head size, as fractions of a square.
const HEAD_LENGTH: f32 = 0.45;
const HEAD_HALF_WIDTH: f32 = 0.3;

/// A square's place on screen: column from the left, row from the top.
pub type Cell = (u8, u8);

/// A shaft through its points that stops where the head begins, and the head's three corners.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrowShape {
    pub shaft: Vec<(f32, f32)>,
    pub head: [(f32, f32); 3],
}

/// An arrow that is neither straight nor diagonal bends: along its longer leg first, then to the target.
pub fn bend_corner(
    (from_column, from_row): Cell,
    (to_column, to_row): Cell,
    start: (f32, f32),
    end: (f32, f32),
) -> Option<(f32, f32)> {
    let (columns, rows) = (from_column.abs_diff(to_column), from_row.abs_diff(to_row));
    if columns == 0 || rows == 0 || columns == rows {
        None
    } else if columns > rows {
        Some((end.0, start.1))
    } else {
        Some((start.0, end.1))
    }
}

pub fn centre((column, row): Cell, cell: f32, (left, top): (f32, f32)) -> (f32, f32) {
    (
        left + (f32::from(column) + 0.5) * cell,
        top + (f32::from(row) + 0.5) * cell,
    )
}

/// The head points along the last leg: straight from `from`, or out of the bend.
pub fn arrow_shape(
    from: (f32, f32),
    to: (f32, f32),
    corner: Option<(f32, f32)>,
    cell: f32,
) -> ArrowShape {
    let last_leg_start = corner.unwrap_or(from);
    let (dx, dy) = (to.0 - last_leg_start.0, to.1 - last_leg_start.1);
    let length = (dx * dx + dy * dy).sqrt();
    let (ux, uy) = (dx / length, dy / length);
    let head_length = HEAD_LENGTH * cell;
    let half_width = HEAD_HALF_WIDTH * cell;
    let neck = (to.0 - ux * head_length, to.1 - uy * head_length);
    ArrowShape {
        shaft: [Some(from), corner, Some(neck)]
            .into_iter()
            .flatten()
            .collect(),
        head: [
            to,
            (neck.0 - uy * half_width, neck.1 + ux * half_width),
            (neck.0 + uy * half_width, neck.1 - ux * half_width),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{arrow_shape, bend_corner, centre};

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 0.001 && (a.1 - b.1).abs() < 0.001
    }

    #[test]
    fn squares_are_found_by_their_centres() {
        assert!(close(centre((0, 0), 10.0, (0.0, 0.0)), (5.0, 5.0)));
        assert!(close(centre((7, 7), 10.0, (100.0, 0.0)), (175.0, 75.0)));
    }

    #[test]
    fn a_rightward_arrow_ends_at_its_tip_with_the_head_across_it() {
        let shape = arrow_shape((0.0, 0.0), (100.0, 0.0), None, 10.0);
        assert!(close(shape.head[0], (100.0, 0.0)));
        assert!(close(shape.shaft[1], (95.5, 0.0)));
        assert!(close(shape.head[1], (95.5, 3.0)));
        assert!(close(shape.head[2], (95.5, -3.0)));
    }

    #[test]
    fn a_knight_arrow_turns_after_its_long_leg() {
        // b1 to c3 as seen from White: one column right, two rows up.
        let (start, end) = ((15.0, 75.0), (25.0, 55.0));
        let corner = bend_corner((1, 7), (2, 5), start, end);
        assert_eq!(corner, Some((15.0, 55.0)));
        let shape = arrow_shape(start, end, corner, 10.0);
        assert_eq!(shape.shaft.len(), 3);
        assert!(close(shape.shaft[1], (15.0, 55.0)));
        // The last leg runs right, so the head points right.
        assert!(close(shape.shaft[2], (20.5, 55.0)));
    }

    #[test]
    fn any_uneven_arrow_bends_after_its_longer_leg() {
        // e2 to f5: three rows up, one column right, so up first.
        assert_eq!(
            bend_corner((4, 6), (5, 3), (45.0, 65.0), (55.0, 35.0)),
            Some((45.0, 35.0))
        );
        // a1 to d2: three columns right, one row up, so right first.
        assert_eq!(
            bend_corner((0, 7), (3, 6), (5.0, 75.0), (35.0, 65.0)),
            Some((35.0, 75.0))
        );
    }

    #[test]
    fn straight_and_diagonal_arrows_stay_straight() {
        assert_eq!(bend_corner((4, 6), (4, 4), (0.0, 0.0), (0.0, 0.0)), None);
        assert_eq!(bend_corner((0, 6), (5, 6), (0.0, 0.0), (0.0, 0.0)), None);
        assert_eq!(bend_corner((0, 0), (2, 2), (0.0, 0.0), (0.0, 0.0)), None);
    }
}
