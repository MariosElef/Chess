//! Pieces gliding from one square to another when the board moves a move forward or back.
#![expect(
    clippy::float_arithmetic,
    reason = "a sliding piece sits part of the way between two squares"
)]

use std::time::Duration;

use gpui_kit::*;

use crate::services::board::Cell;

/// How long a piece takes to slide, about as long as on chess.com.
pub const SLIDE_TIME: Duration = Duration::from_millis(180);

/// One piece on its way: its picture, and the cells it leaves and reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlidingPiece {
    pub picture: &'static str,
    pub from: Cell,
    pub to: Cell,
}

/// The pieces sliding, over the board; `run` names this slide so a new one starts afresh.
pub fn sliding_layer(pieces: Vec<SlidingPiece>, run: usize) -> Div {
    div()
        .absolute()
        .inset_0()
        .children(pieces.into_iter().enumerate().map(|(index, piece)| {
            let id = ElementId::NamedInteger("slide".into(), (run * 4 + index) as u64);
            let (from, to) = (piece.from, piece.to);
            img(piece.picture)
                .absolute()
                .size(relative(1. / 8.))
                .with_animation(
                    id,
                    Animation::new(SLIDE_TIME).with_easing(ease_in_out),
                    move |this, progress| {
                        this.left(relative(along(from.0, to.0, progress)))
                            .top(relative(along(from.1, to.1, progress)))
                    },
                )
        }))
}

/// Where between two cells' edges the piece is, as a share of the board.
fn along(from: u8, to: u8, progress: f32) -> f32 {
    let (from, to) = (f32::from(from) / 8., f32::from(to) / 8.);
    from + (to - from) * progress
}
