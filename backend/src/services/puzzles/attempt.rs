use chess_domain::{RatedResult, Rating, Score};
use uuid::Uuid;

use super::rating::{PUZZLE_CATEGORY, from_row, shown, to_row};
use crate::{
    errors::{AppError, FieldError},
    models::puzzles::{
        attempt::{CreateAttemptModel, GetAttemptModel, NewAttempt},
        rating::PuzzleStrength,
    },
    models::rating::RatingSource,
    repositories::{
        database::Db,
        puzzles::{attempt, puzzle},
        rating,
    },
};

/// How sure a puzzle's own rating is taken to be; puzzles are rated by thousands of plays.
const PUZZLE_VOLATILITY: f64 = 0.06;

/// Puzzle attempts: remembered so no puzzle comes twice, and in Practice, moving the rating.
#[derive(Clone)]
pub struct AttemptService {
    db: Db,
}

impl AttemptService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Records the attempt and, when rated, the new rating; a guest's attempt keeps nothing.
    pub async fn record(
        &self,
        user_id: Uuid,
        guest: bool,
        puzzle_id: String,
        body: CreateAttemptModel,
    ) -> Result<GetAttemptModel, AppError> {
        if body.duration_ms < 0 {
            return Err(AppError::fields(vec![FieldError::new(
                "/duration_ms",
                "A duration is never negative",
            )]));
        }
        let nothing_kept = GetAttemptModel { rating: None };
        if guest {
            return Ok(nothing_kept);
        }
        let mut tx = self.db.begin().await?;
        let Some(strength) = puzzle::strength(&mut *tx, &puzzle_id).await? else {
            return Ok(nothing_kept);
        };
        let new_attempt = NewAttempt {
            id: Uuid::now_v7(),
            user_id,
            puzzle_id,
            solved: body.solved,
            duration_ms: body.duration_ms,
            rated: body.rated,
        };
        attempt::insert(&mut *tx, &new_attempt).await?;
        let mut new_rating = None;
        if body.rated {
            let before = rating::get_for_update(&mut tx, user_id, PUZZLE_CATEGORY)
                .await?
                .map_or(Rating::NEW, from_row);
            let after = before.after(&[RatedResult {
                opponent: puzzle_as_player(strength),
                score: if body.solved { Score::Win } else { Score::Loss },
            }]);
            rating::record_result(
                &mut tx,
                user_id,
                PUZZLE_CATEGORY,
                RatingSource::PuzzleAttempt(new_attempt.id),
                to_row(after),
            )
            .await?;
            new_rating = Some(shown(after, Some(before)));
        }
        tx.commit().await?;
        let response = GetAttemptModel { rating: new_rating };
        Ok(response)
    }
}

/// The puzzle as the opponent the player beat or lost to.
fn puzzle_as_player(strength: PuzzleStrength) -> Rating {
    Rating {
        rating: f64::from(strength.rating),
        deviation: f64::from(strength.rating_deviation),
        volatility: PUZZLE_VOLATILITY,
    }
}
