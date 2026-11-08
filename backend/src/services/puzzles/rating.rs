//! Between the stored rating and the domain's Glicko-2 one, and what the player is shown.
use chess_domain::Rating;

use crate::models::puzzles::rating::{PuzzleRatingModel, RatingRow};

/// Puzzle ratings are kept in this category of `play.ratings`.
pub const PUZZLE_CATEGORY: &str = "puzzle";

pub fn from_row(row: RatingRow) -> Rating {
    Rating {
        rating: row.rating,
        deviation: row.deviation,
        volatility: row.volatility,
    }
}

pub fn to_row(rating: Rating) -> RatingRow {
    RatingRow {
        rating: rating.rating,
        deviation: rating.deviation,
        volatility: rating.volatility,
    }
}

/// Rounded for the player; `before` gives the change the last result made.
pub fn shown(rating: Rating, before: Option<Rating>) -> PuzzleRatingModel {
    PuzzleRatingModel {
        rating: whole(rating.rating),
        deviation: whole(rating.deviation),
        provisional: rating.is_provisional(),
        change: before.map(|before| whole(rating.rating) - whole(before.rating)),
    }
}

fn whole(value: f64) -> i32 {
    value.round() as i32
}
