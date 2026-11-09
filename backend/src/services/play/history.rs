use std::collections::HashMap;

use uuid::Uuid;

use crate::{
    enums::play::{Color, GameResult, PlayerResult},
    errors::{AppError, ChessCode},
    models::{
        play::history::{GameRecordModel, PlayedGameModel, PlayedGameRow},
        rating::RatingChangeRow,
    },
    repositories::{database::Db, play::history, rating},
    services::rating::whole,
};

/// How many games a page of history holds.
const PAGE: i64 = 10;

/// A player's finished online games: a page at a time, or one with its moves to look at again.
#[derive(Clone)]
pub struct GameHistoryService {
    db: Db,
}

impl GameHistoryService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// The player's finished games before `before`, the latest first.
    pub async fn page(
        &self,
        player: Uuid,
        before: Option<Uuid>,
    ) -> Result<Vec<PlayedGameModel>, AppError> {
        let rows = history::page(&self.db, player, (before, PAGE)).await?;
        let ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
        let changes = changes_by_source(rating::changes(&self.db, player, &ids).await?);
        Ok(rows
            .into_iter()
            .filter_map(|row| played_game(player, row, &changes))
            .collect())
    }

    /// One of the player's finished games with its moves; another's is not found.
    pub async fn one(&self, player: Uuid, game_id: Uuid) -> Result<GameRecordModel, AppError> {
        let row = history::one(&self.db, player, game_id)
            .await?
            .ok_or(ChessCode::GameNotFound)?;
        let changes = changes_by_source(rating::changes(&self.db, player, &[game_id]).await?);
        let game = played_game(player, row, &changes).ok_or(ChessCode::GameNotFound)?;
        let moves = history::moves(&self.db, game_id).await?;
        Ok(GameRecordModel { game, moves })
    }
}

/// The rating changes, by the game they followed.
fn changes_by_source(rows: Vec<RatingChangeRow>) -> HashMap<Uuid, RatingChangeRow> {
    rows.into_iter().map(|row| (row.source_id, row)).collect()
}

/// The game as the player saw it; `None` if its stored outcome is not one this server writes.
fn played_game(
    player: Uuid,
    row: PlayedGameRow,
    changes: &HashMap<Uuid, RatingChangeRow>,
) -> Option<PlayedGameModel> {
    let outcome = GameResult::from_key(&row.outcome)?;
    let winner = row.winner.as_deref().and_then(Color::from_key);
    let (color, opponent) = if row.white_id == player {
        (Color::White, row.black_name)
    } else {
        (Color::Black, row.white_name)
    };
    let change = changes.get(&row.id);
    Some(PlayedGameModel {
        id: row.id,
        color,
        opponent,
        result: result_for(color, winner),
        outcome,
        category: row.category,
        initial_s: row.initial_s,
        increment_s: row.increment_s,
        rated: row.rated,
        plies: row.ply_count,
        ago_s: row.ago_s,
        rating: change.map(|change| whole(change.rating)),
        rating_change: change.map(|change| whole(change.rating) - whole(change.before)),
    })
}

/// How it went for `me`, a side or a player, given who won: none for a draw.
pub fn result_for<T: PartialEq>(me: T, winner: Option<T>) -> PlayerResult {
    match winner {
        None => PlayerResult::Drawn,
        Some(winner) if winner == me => PlayerResult::Won,
        Some(_) => PlayerResult::Lost,
    }
}

#[cfg(test)]
mod tests {
    use super::result_for;
    use crate::enums::play::{Color, PlayerResult};

    #[test]
    fn a_result_is_told_from_the_players_side() {
        assert_eq!(
            result_for(Color::White, Some(Color::White)),
            PlayerResult::Won
        );
        assert_eq!(
            result_for(Color::Black, Some(Color::White)),
            PlayerResult::Lost
        );
        assert_eq!(result_for(Color::Black, None), PlayerResult::Drawn);
    }
}
