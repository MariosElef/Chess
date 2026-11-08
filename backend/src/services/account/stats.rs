use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{
        account::{
            CategoryRatingRow, GameResultsRow, GetStatsModel, HistoryRow, ModeStatsModel,
            PuzzleStatsModel,
        },
        rating::RatingRow,
    },
    repositories::{account::stats, database::Db, puzzles::rush_run},
    services::{
        puzzles::{
            puzzle::best_rush,
            rating::{BATTLE_CATEGORY, PUZZLE_CATEGORY},
        },
        rating::{from_row, whole},
    },
};

/// The player's ratings in every mode and in puzzles, with their games and attempts.
#[derive(Clone)]
pub struct StatsService {
    db: Db,
}

impl StatsService {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub async fn of(&self, user_id: Uuid) -> Result<GetStatsModel, AppError> {
        let ratings = stats::ratings(&self.db, user_id).await?;
        let results = stats::game_results(&self.db, user_id).await?;
        let counts = stats::puzzle_counts(&self.db, user_id).await?;
        let bests = rush_run::bests(&self.db, user_id).await?;
        let history = stats::history(&self.db, user_id, (GRAPHED, BATTLE_CATEGORY)).await?;
        let mut puzzles = PuzzleStatsModel {
            rating: None,
            provisional: false,
            best: None,
            attempted: counts.attempted,
            solved: counts.solved,
            best_rush: best_rush(bests),
            battle_rating: None,
            battle_provisional: false,
            history: recent(&history, PUZZLE_CATEGORY),
        };
        let mut modes = Vec::new();
        for row in ratings {
            let (rating, provisional) = shown(&row);
            let best = row.best.map(whole);
            if row.category == PUZZLE_CATEGORY {
                puzzles.rating = rating;
                puzzles.provisional = provisional;
                puzzles.best = best;
            } else if row.category == BATTLE_CATEGORY {
                puzzles.battle_rating = rating;
                puzzles.battle_provisional = provisional;
            } else {
                let graph = recent(&history, &row.category);
                modes.push(mode(
                    row.category,
                    (rating, provisional, best),
                    (&results, graph),
                ));
            }
        }
        Ok(GetStatsModel { modes, puzzles })
    }
}

/// The rating rounded, and whether it is still provisional; `None` before the first result.
fn shown(row: &CategoryRatingRow) -> (Option<i32>, bool) {
    let (Some(rating), Some(deviation), Some(volatility)) =
        (row.rating, row.deviation, row.volatility)
    else {
        return (None, false);
    };
    let rating = from_row(RatingRow {
        rating,
        deviation,
        volatility,
    });
    (Some(whole(rating.rating)), rating.is_provisional())
}

/// How many of a category's latest ratings its graph shows.
const GRAPHED: i64 = 30;

/// The category's ratings from the history, rounded and oldest first.
fn recent(history: &[HistoryRow], category: &str) -> Vec<i32> {
    history
        .iter()
        .filter(|row| row.category == category)
        .map(|row| whole(row.rating))
        .collect()
}

fn mode(
    category: String,
    (rating, provisional, best): (Option<i32>, bool, Option<i32>),
    (results, history): (&[GameResultsRow], Vec<i32>),
) -> ModeStatsModel {
    let played = results.iter().find(|result| result.category == category);
    let (games, wins, draws) = played.map_or((0, 0, 0), |r| (r.games, r.wins, r.draws));
    ModeStatsModel {
        category,
        rating,
        provisional,
        best,
        games,
        wins,
        losses: games - wins - draws,
        draws,
        history,
    }
}
