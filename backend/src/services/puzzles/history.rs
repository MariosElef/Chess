use std::collections::HashMap;

use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{
        puzzles::history::{AttemptRow, BattleRow, PlayedAttemptModel, PlayedBattleModel},
        rating::RatingChangeRow,
    },
    repositories::{database::Db, puzzles::history, rating},
    services::{play::history::result_for, rating::whole},
};

/// How many battles or attempts a page of history holds.
const PAGE: i64 = 10;

/// A player's finished puzzle battles and puzzle attempts, a page at a time.
#[derive(Clone)]
pub struct PuzzleHistoryService {
    db: Db,
}

impl PuzzleHistoryService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// The player's finished battles before `before`, the latest first.
    pub async fn battles(
        &self,
        player: Uuid,
        before: Option<Uuid>,
    ) -> Result<Vec<PlayedBattleModel>, AppError> {
        let rows = history::battles(&self.db, player, (before, PAGE)).await?;
        let changes = self.changes(player, rows.iter().map(|row| row.id)).await?;
        Ok(rows
            .into_iter()
            .map(|row| played_battle(player, row, &changes))
            .collect())
    }

    /// The player's puzzle attempts before `before`, the latest first.
    pub async fn attempts(
        &self,
        player: Uuid,
        before: Option<Uuid>,
    ) -> Result<Vec<PlayedAttemptModel>, AppError> {
        let rows = history::attempts(&self.db, player, (before, PAGE)).await?;
        let changes = self.changes(player, rows.iter().map(|row| row.id)).await?;
        Ok(rows
            .into_iter()
            .map(|row| played_attempt(row, &changes))
            .collect())
    }

    /// The rating changes that followed `sources`, by source.
    async fn changes(
        &self,
        player: Uuid,
        sources: impl Iterator<Item = Uuid>,
    ) -> Result<HashMap<Uuid, RatingChangeRow>, AppError> {
        let sources: Vec<Uuid> = sources.collect();
        let rows = rating::changes(&self.db, player, &sources).await?;
        Ok(rows.into_iter().map(|row| (row.source_id, row)).collect())
    }
}

/// The battle as the player saw it: their score first, and how it went for them.
fn played_battle(
    player: Uuid,
    row: BattleRow,
    changes: &HashMap<Uuid, RatingChangeRow>,
) -> PlayedBattleModel {
    let (my_score, their_score) = if row.first_id == player {
        (row.first_score, row.second_score)
    } else {
        (row.second_score, row.first_score)
    };
    let result = result_for(player, row.winner_id);
    let (rating, rating_change) = shown_change(changes.get(&row.id));
    PlayedBattleModel {
        id: row.id,
        opponent: row.opponent,
        my_score,
        their_score,
        result,
        rated: row.rated,
        ago_s: row.ago_s,
        rating,
        rating_change,
    }
}

fn played_attempt(row: AttemptRow, changes: &HashMap<Uuid, RatingChangeRow>) -> PlayedAttemptModel {
    let (rating, rating_change) = shown_change(changes.get(&row.id));
    PlayedAttemptModel {
        id: row.id,
        puzzle_id: row.puzzle_id,
        puzzle_rating: row.puzzle_rating,
        solved: row.solved,
        duration_ms: row.duration_ms,
        rated: row.rated,
        ago_s: row.ago_s,
        rating,
        rating_change,
    }
}

/// The rating after, and how far it moved, both rounded as a person reads them.
fn shown_change(change: Option<&RatingChangeRow>) -> (Option<i32>, Option<i32>) {
    (
        change.map(|change| whole(change.rating)),
        change.map(|change| whole(change.rating) - whole(change.before)),
    )
}
