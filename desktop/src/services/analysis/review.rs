//! Game review: every move of a game classified as chess.com's Game Review does, and each
//! side's accuracy.
//!
//! Chess.com grades a move by the expected points it loses against the engine's best move (its
//! "Classification V2"). Their curve from evaluation to expected points depends on rating and is
//! not published, nor are the exact conditions for Brilliant, Great and Miss; this uses
//! Lichess's published curve and accuracy formula, and cut-offs that follow chess.com's wording.
mod accuracy;
mod classify;
mod game_rating;
mod opening_book;
mod review_engines;
mod review_run;
mod sacrifice;

use chess_domain::Side;

pub use review_engines::review_engines;
pub use review_run::ReviewRun;

use crate::{enums::analysis::MoveClass, models::analysis::GameReview};

/// How many of `side`'s moves the review gave `class`.
pub fn class_count(review: &GameReview, side: Side, class: MoveClass) -> usize {
    review
        .moves
        .iter()
        .filter(|reviewed| reviewed.mover == side && reviewed.class == class)
        .count()
}
