use uuid::Uuid;

use crate::{
    errors::{AppError, FieldError},
    models::puzzles::rush::{CreateRushModel, GetRushModel, NewRushRun},
    repositories::{database::Db, puzzles::rush_run},
};

/// Finished Puzzle Rushes and the best of each kind.
#[derive(Clone)]
pub struct RushService {
    db: Db,
}

impl RushService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Stores the rush and says whether it beat the player's best of its kind; not for a guest.
    pub async fn record(
        &self,
        user_id: Uuid,
        guest: bool,
        body: CreateRushModel,
    ) -> Result<GetRushModel, AppError> {
        if body.score < 0 {
            return Err(AppError::fields(vec![FieldError::new(
                "/score",
                "A score is never negative",
            )]));
        }
        if guest {
            return Ok(GetRushModel {
                best: body.score,
                new_best: false,
            });
        }
        let previous = rush_run::best(&self.db, user_id, body.kind).await?;
        let run = NewRushRun {
            id: Uuid::now_v7(),
            user_id,
            kind: body.kind,
            score: body.score,
        };
        rush_run::insert(&self.db, &run).await?;
        let response = GetRushModel {
            best: previous.map_or(body.score, |best| best.max(body.score)),
            new_best: previous.is_none_or(|best| body.score > best),
        };
        Ok(response)
    }
}
