//! The puzzle rating's category, and how the player is shown it.
use chess_domain::Rating;

pub use crate::services::rating::{from_row, to_row};
use crate::{models::puzzles::rating::PuzzleRatingModel, services::rating::whole};

pub use crate::enums::play::{BATTLE_CATEGORY, PUZZLE_CATEGORY};

/// Rounded for the player; `before` gives the change the last result made.
pub fn shown(rating: Rating, before: Option<Rating>) -> PuzzleRatingModel {
    PuzzleRatingModel {
        rating: whole(rating.rating),
        deviation: whole(rating.deviation),
        provisional: rating.is_provisional(),
        change: before.map(|before| whole(rating.rating) - whole(before.rating)),
    }
}
