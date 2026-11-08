//! Between a stored rating and the domain's Glicko-2 one, for puzzles and games alike.
use chess_domain::Rating;

use crate::models::rating::RatingRow;

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

/// A rating as a person reads it.
pub fn whole(value: f64) -> i32 {
    value.round() as i32
}
