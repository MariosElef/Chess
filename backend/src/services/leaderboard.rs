use uuid::Uuid;

use crate::{
    enums::play::RatingBoard,
    errors::AppError,
    models::rating::{LeaderModel, LeaderRow, LeaderboardModel, RatingRow},
    repositories::{database::Db, rating},
    services::rating::{from_row, whole},
};

/// How many places a page of a leaderboard holds.
const PAGE: usize = 50;

/// Everyone's ratings, ranked: a leaderboard per category, a page at a time.
#[derive(Clone)]
pub struct LeaderboardService {
    db: Db,
}

impl LeaderboardService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// The board's places from `offset` on, with the asker's own; a guest is on no board.
    pub async fn board(
        &self,
        board: RatingBoard,
        (asker, guest): (Uuid, bool),
        offset: i64,
    ) -> Result<LeaderboardModel, AppError> {
        // One more than a page, to tell whether another follows.
        let asked = i64::try_from(PAGE + 1).unwrap_or(i64::MAX);
        let mut rows = self.places(board, (offset.max(0), asked)).await?;
        let more = rows.len() > PAGE;
        rows.truncate(PAGE);
        let me = if guest {
            None
        } else {
            self.place_of(board, asker).await?
        };
        Ok(LeaderboardModel {
            rows: rows.into_iter().map(leader).collect(),
            me: me.map(leader),
            more,
        })
    }
}

impl LeaderboardService {
    /// Up to `limit` of the board's places from `offset` on.
    async fn places(
        &self,
        board: RatingBoard,
        window: (i64, i64),
    ) -> Result<Vec<LeaderRow>, AppError> {
        let rows = match board.category() {
            Some(category) => rating::leaderboard(&self.db, &category, window).await?,
            None => rating::average_leaderboard(&self.db, window).await?,
        };
        Ok(rows)
    }

    /// The player's own place on the board, if they are on it.
    async fn place_of(
        &self,
        board: RatingBoard,
        player: Uuid,
    ) -> Result<Option<LeaderRow>, AppError> {
        let row = match board.category() {
            Some(category) => rating::leaderboard_place(&self.db, &category, player).await?,
            None => rating::average_leaderboard_place(&self.db, player).await?,
        };
        Ok(row)
    }
}

/// A place as the board shows it: the rating rounded, and whether it is still provisional.
fn leader(row: LeaderRow) -> LeaderModel {
    let rating = from_row(RatingRow {
        rating: row.rating,
        deviation: row.deviation,
        volatility: row.volatility,
    });
    LeaderModel {
        place: row.place,
        id: row.user_id,
        username: row.username,
        avatar: row.avatar,
        rating: whole(rating.rating),
        provisional: rating.is_provisional(),
        played: row.games_played,
    }
}
