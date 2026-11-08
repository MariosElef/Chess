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
    players: ((Uuid, Rating), (Uuid, Rating)),
    category: &str,
    winner: Option<Side>,
) -> Result<RatingChanges, AppError> {
    rate_pair(db, RatingSource::Game(game_id), players, category, winner).await
}

/// Moves two players' ratings in `category` after a contest between them, each against the
/// other's rating from before it; `winner` names a side, white being the first player. Each
/// player comes with their rating when the contest began.
pub async fn rate_pair(
    db: &Db,
    source: RatingSource,
    ((white, white_at_start), (black, black_at_start)): ((Uuid, Rating), (Uuid, Rating)),
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
    // The players before their ratings, the order closing an account takes them in.
    rating::lock_players(&mut tx, &[first, second]).await?;
    let first_rating = rating::get_for_update(&mut tx, first, category).await?;
    let second_rating = rating::get_for_update(&mut tx, second, category).await?;
    let (white_row, black_row) = if white < black {
        (first_rating, second_rating)
    } else {
        (second_rating, first_rating)
    };
    // A rating gone since the start belongs to an account that closed: the other player is
    // still rated against what it was.
    let white_before = white_row.map_or(white_at_start, from_row);
    let black_before = black_row.map_or(black_at_start, from_row);
    let (white_score, black_score) = scores(winner);
    let white_after = white_before.after(&[RatedResult {
        opponent: black_before,
        score: white_score,
    }]);
    let black_after = black_before.after(&[RatedResult {
        opponent: white_before,
        score: black_score,
    }]);
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
