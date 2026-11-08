//! The puzzle rating's category, and how the player is shown it.
use chess_domain::Rating;

pub use crate::services::rating::{from_row, to_row};
use crate::{models::puzzles::rating::PuzzleRatingModel, services::rating::whole};

/// Puzzle ratings are kept in this category of `play.ratings`.
pub const PUZZLE_CATEGORY: &str = "puzzle";
/// The category Puzzle Battle's rating moves in.
pub const BATTLE_CATEGORY: &str = "puzzle_battle";

/// Rounded for the player; `before` gives the change the last result made.
pub fn shown(rating: Rating, before: Option<Rating>) -> PuzzleRatingModel {
    PuzzleRatingModel {
        rating: whole(rating.rating),
        deviation: whole(rating.deviation),
        provisional: rating.is_provisional(),
        change: before.map(|before| whole(rating.rating) - whole(before.rating)),
    }
}
