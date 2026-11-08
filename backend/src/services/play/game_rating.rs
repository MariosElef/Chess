use chess_domain::{RatedResult, Rating, Score, Side};
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{play::game::RatingChanges, rating::RatingSource},
    repositories::{database::Db, rating},
    services::rating::{from_row, to_row, whole},
};

/// Moves both players' ratings in the game's category after a rated game, each against the
/// other's rating from before it, and records the game in their rating history.
pub async fn rate_game(
    db: &Db,
    game_id: Uuid,
    (white, black): (Uuid, Uuid),
    category: &str,
    winner: Option<Side>,
) -> Result<RatingChanges, AppError> {
    let mut tx = db.begin().await?;
    // Locked in id order, so two games ending at once can never wait on each other.
    let (first, second) = if white < black {
        (white, black)
    } else {
        (black, white)
    };
    let first_rating = rating::get_for_update(&mut tx, first, category).await?;
    let second_rating = rating::get_for_update(&mut tx, second, category).await?;
    let (white_row, black_row) = if white < black {
        (first_rating, second_rating)
    } else {
        (second_rating, first_rating)
    };
    let white_before = white_row.map_or(Rating::NEW, from_row);
    let black_before = black_row.map_or(Rating::NEW, from_row);
    let (white_score, black_score) = scores(winner);
    let white_after = white_before.after(&[RatedResult {
        opponent: black_before,
        score: white_score,
    }]);
    let black_after = black_before.after(&[RatedResult {
        opponent: white_before,
        score: black_score,
    }]);
    let source = RatingSource::Game(game_id);
    rating::record_result(&mut tx, white, category, source, to_row(white_after)).await?;
    rating::record_result(&mut tx, black, category, source, to_row(black_after)).await?;
    tx.commit().await?;
    let response = RatingChanges {
        white: whole(white_after.rating) - whole(white_before.rating),
        black: whole(black_after.rating) - whole(black_before.rating),
    };
    Ok(response)
}

fn scores(winner: Option<Side>) -> (Score, Score) {
    match winner {
        Some(Side::White) => (Score::Win, Score::Loss),
        Some(Side::Black) => (Score::Loss, Score::Win),
        None => (Score::Draw, Score::Draw),
    }
}
