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
        let category = board.key();
        // One more than a page, to tell whether another follows.
        let asked = i64::try_from(PAGE + 1).unwrap_or(i64::MAX);
        let mut rows = rating::leaderboard(&self.db, &category, (offset.max(0), asked)).await?;
        let more = rows.len() > PAGE;
        rows.truncate(PAGE);
        let me = if guest {
            None
        } else {
            rating::leaderboard_place(&self.db, &category, asker).await?
        };
        Ok(LeaderboardModel {
            rows: rows.into_iter().map(leader).collect(),
            me: me.map(leader),
            more,
        })
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
