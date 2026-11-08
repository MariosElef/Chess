//! The piece the player holds: it glides from its square until its centre is on the cursor, then
//! follows it, with the square under it outlined.
use std::time::Duration;

use gpui_kit::{prelude::FluentBuilder as _, *};

/// How long a pressed piece takes to glide from its square to the cursor.
const CENTRE_GLIDE: Duration = Duration::from_millis(110);
/// The outline of the square the held piece is over.
const OUTLINE: Pixels = px(3.);

/// The held piece, in points from the board's top-left corner.
#[derive(IntoElement)]
pub struct HeldPiece {
    picture: &'static str,
    /// One square's width, the piece's size.
    size: Pixels,
    /// The centre of the square it was picked up from, and the cursor now.
    from: Point<Pixels>,
    cursor: Point<Pixels>,
    /// The top-left corner of the square under it, once it is being dragged over the board.
    over: Option<Point<Pixels>>,
    /// Which hold this is, so each one glides afresh.
    run: usize,
}

impl HeldPiece {
    pub fn new(
        (picture, size): (&'static str, Pixels),
        (from, cursor): (Point<Pixels>, Point<Pixels>),
        over: Option<Point<Pixels>>,
        run: usize,
    ) -> Self {
        Self {
            picture,
            size,
            from,
            cursor,
            over,
            run,
        }
    }
}

impl RenderOnce for HeldPiece {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (size, from, cursor) = (self.size, self.from, self.cursor);
        let half = size / 2.;
        div()
            .absolute()
            .inset_0()
            .when_some(self.over, |this, corner| {
                this.child(
                    div()
                        .debug_selector(|| "hover-outline".to_owned())
                        .absolute()
                        .left(corner.x)
                        .top(corner.y)
                        .size(size)
                        .border(OUTLINE)
                        .border_color(hsla(0., 0., 1., 0.65)),
                )
            })
            .child(
                div()
                    .absolute()
                    .size(size)
                    .child(
                        img(self.picture)
                            .size_full()
                            .debug_selector(|| "pressed-piece".to_owned()),
                    )
                    .with_animation(
                        ElementId::NamedInteger("held".into(), self.run as u64),
                        Animation::new(CENTRE_GLIDE).with_easing(ease_out_quint()),
                        move |this, progress| {
                            let centre = from + (cursor - from) * progress;
                            this.left(centre.x - half).top(centre.y - half)
                        },
                    ),
            )
    }
}
