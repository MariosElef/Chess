use anyhow::anyhow;
use chess_domain::{Move, Position};
use uuid::Uuid;

use super::rating::{BATTLE_CATEGORY, PUZZLE_CATEGORY, from_row, shown};
use crate::{
    clients::lichess::LichessClient,
    enums::puzzles::RushKind,
    errors::{AppError, ChessCode, FieldError, Fields},
    models::puzzles::{
        battle::BattleSummaryModel,
        puzzle::{GetPuzzleModel, PuzzleQuery, PuzzleRow},
        rush::BestRushRow,
        summary::{BestRushModel, GetPuzzleSummaryModel},
    },
    repositories::{
        database::Db,
        puzzles::{battle, puzzle, rush_run},
        rating,
    },
};

/// The widest range a player may ask for, and what a missing end means.
const LOWEST_RATING: i32 = 400;
const HIGHEST_RATING: i32 = 3200;
/// A battle's ladder: how many puzzles, the first one's rating, and how much harder each next.
const LADDER_LENGTH: usize = 40;
const LADDER_START: i32 = 600;
const LADDER_STEP: i32 = 40;
/// Puzzle ids are Lichess's: five letters and digits.
const ID_ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Puzzles to solve: from the imported Lichess database, or straight from Lichess before the
/// database has been imported.
#[derive(Clone)]
pub struct PuzzleService {
    db: Db,
    lichess: LichessClient,
}

impl PuzzleService {
    pub fn new(db: Db, lichess: LichessClient) -> Self {
        Self { db, lichess }
    }

    /// A random puzzle rated within the range asked for, one the player has not attempted.
    pub async fn next(
        &self,
        query: PuzzleQuery,
        user_id: Uuid,
    ) -> Result<GetPuzzleModel, AppError> {
        let (min, max) = checked_range(query)?;
        if let Some(row) = self.random_in_range(min, max, user_id).await? {
            return opened(row);
        }
        if puzzle::any(&self.db).await? {
            return Err(ChessCode::NoPuzzleInRange.into());
        }
        let response = self
            .lichess
            .next_puzzle(lichess_difficulty(min, max))
            .await?;
        Ok(response)
    }

    /// The first puzzle after a random point in the range; past the range's last puzzle, the
    /// range's first.
    async fn random_in_range(
        &self,
        min: i32,
        max: i32,
        user_id: Uuid,
    ) -> Result<Option<PuzzleRow>, AppError> {
        let from = (rand::random_range(min..=max), random_id_start());
        let range = (min, max);
        if let Some(row) = puzzle::first_from(&self.db, range, (from.0, &from.1), user_id).await? {
            return Ok(Some(row));
        }
        Ok(puzzle::first_from(&self.db, range, (min, ""), user_id).await?)
    }

    /// The puzzle rating and best rushes, ever and today, for the Puzzles menu; a guest keeps
    /// none of them.
    pub async fn summary(
        &self,
        user_id: Uuid,
        guest: bool,
    ) -> Result<GetPuzzleSummaryModel, AppError> {
        if guest {
            return Ok(GetPuzzleSummaryModel {
                guest,
                rating: None,
                best_rush: BestRushModel::default(),
                best_today: BestRushModel::default(),
                battle: BattleSummaryModel::default(),
            });
        }
        let rating = rating::get(&self.db, user_id, PUZZLE_CATEGORY)
            .await?
            .map(|row| shown(from_row(row), None));
        let bests = rush_run::bests(&self.db, user_id).await?;
        let today = rush_run::bests_today(&self.db, user_id).await?;
        let battle_rating = rating::get(&self.db, user_id, BATTLE_CATEGORY)
            .await?
            .map(|row| shown(from_row(row), None));
        let wins_this_month = battle::wins_this_month(&self.db, user_id).await?;
        let response = GetPuzzleSummaryModel {
            guest,
            rating,
            best_rush: best_rush(bests),
            best_today: best_rush(today),
            battle: BattleSummaryModel {
                rating: battle_rating,
                wins_this_month,
            },
        };
        Ok(response)
    }
}

/// Each end defaults to the widest; a range that is backwards or out of bounds is refused.
fn checked_range(query: PuzzleQuery) -> Result<(i32, i32), AppError> {
    let mut fields = Fields::default();
    let min = fields.check(in_bounds(
        query.min_rating.unwrap_or(LOWEST_RATING),
        "min_rating",
    ));
    let max = fields.check(in_bounds(
        query.max_rating.unwrap_or(HIGHEST_RATING),
        "max_rating",
    ));
    fields.finish()?;
    if min > max {
        return Err(AppError::fields(vec![FieldError::new(
            "max_rating",
            "The highest rating is below the lowest",
        )]));
    }
    Ok((min, max))
}

fn in_bounds(rating: i32, name: &str) -> Result<i32, FieldError> {
    if (LOWEST_RATING..=HIGHEST_RATING).contains(&rating) {
        Ok(rating)
    } else {
        Err(FieldError::new(
            name,
            format!("A rating is between {LOWEST_RATING} and {HIGHEST_RATING}"),
        ))
    }
}

/// Each kind's best, from the database's rows.
pub fn best_rush(rows: Vec<BestRushRow>) -> BestRushModel {
    let mut best = BestRushModel::default();
    for row in rows {
        let slot = match RushKind::from_key(&row.kind) {
            Some(RushKind::ThreeMinutes) => &mut best.three_minutes,
            Some(RushKind::FiveMinutes) => &mut best.five_minutes,
            Some(RushKind::Survival) => &mut best.survival,
            None => continue,
        };
        *slot = Some(row.best);
    }
    best
}

/// One random letter or digit: where in a rating's puzzles the walk starts.
fn random_id_start() -> String {
    let index = rand::random_range(0..ID_ALPHABET.len());
    char::from(ID_ALPHABET[index]).to_string()
}

/// The stored puzzle as the app plays it: the opponent's first move made, the player to move.
fn opened(row: PuzzleRow) -> Result<GetPuzzleModel, AppError> {
    let broken = |what: &str| AppError::Unexpected(anyhow!("puzzle {} has a bad {what}", row.id));
    let before: Position = row.fen.parse().map_err(|_| broken("FEN"))?;
    let moves = row
        .moves
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<Vec<Move>, _>>()
        .map_err(|_| broken("move"))?;
    let Some((&first, solution)) = moves.split_first() else {
        return Err(broken("move list"));
    };
    let after = before.play(first).map_err(|_| broken("first move"))?;
    let response = GetPuzzleModel {
        fen: after.to_string(),
        last_move: Some(first.to_string()),
        solution: solution.iter().map(ToString::to_string).collect(),
        id: row.id,
        rating: row.rating,
        plays: row.nb_plays,
        themes: row.themes,
    };
    Ok(response)
}

/// A battle's puzzles, the same for both players: from easy, a little harder each one, so no
/// one ever runs out in three minutes. Needs the imported database.
pub async fn battle_ladder(db: &Db) -> Result<Vec<GetPuzzleModel>, AppError> {
    let mut ladder = Vec::with_capacity(LADDER_LENGTH);
    for step in 0..LADDER_LENGTH {
        let target = LADDER_START + LADDER_STEP * i32::try_from(step).unwrap_or(i32::MAX);
        let range = (target - LADDER_STEP, target + LADDER_STEP);
        let from = (rand::random_range(range.0..=range.1), random_id_start());
        // Nobody's attempts are left out: a nil player has none.
        let row = match puzzle::first_from(db, range, (from.0, &from.1), Uuid::nil()).await? {
            Some(row) => Some(row),
            None => puzzle::first_from(db, range, (range.0, ""), Uuid::nil()).await?,
        };
        // Steps next to each other overlap: a puzzle already on the ladder is not used twice.
        if let Some(row) =
            row.filter(|row| !ladder.iter().any(|kept: &GetPuzzleModel| kept.id == row.id))
        {
            ladder.push(opened(row)?);
        }
    }
    if ladder.is_empty() {
        return Err(ChessCode::NoPuzzleInRange.into());
    }
    Ok(ladder)
}

/// Lichess's five steps are measured against about 1500: the step nearest the range's middle.
fn lichess_difficulty(min: i32, max: i32) -> &'static str {
    match (min + max) / 2 - 1500 {
        ..-450 => "easiest",
        -450..-150 => "easier",
        -150..=150 => "normal",
        151..=450 => "harder",
        _ => "hardest",
    }
}

#[cfg(test)]
mod tests {
    use super::{checked_range, lichess_difficulty, opened};
    use crate::models::puzzles::puzzle::{PuzzleQuery, PuzzleRow};

    #[test]
    fn a_range_defaults_to_the_widest_and_refuses_nonsense() {
        assert_eq!(
            checked_range(PuzzleQuery::default()).ok(),
            Some((400, 3200))
        );
        let backwards = PuzzleQuery {
            min_rating: Some(1600),
            max_rating: Some(1200),
        };
        assert!(checked_range(backwards).is_err());
        let too_low = PuzzleQuery {
            min_rating: Some(100),
            max_rating: None,
        };
        assert!(checked_range(too_low).is_err());
    }

    #[test]
    fn the_opponents_first_move_is_made_before_the_player_sees_the_puzzle() {
        let puzzle = opened(PuzzleRow {
            id: "00008".to_owned(),
            fen: "r6k/pp2r2p/4Rp1Q/3p4/8/1N1P2R1/PqP2bPP/7K b - - 0 24".to_owned(),
            moves: "f2g3 e6e7 b2b1 b3c1 b1c1 h6c1".to_owned(),
            rating: 1819,
            nb_plays: 10446,
            themes: vec!["crushing".to_owned()],
        })
        .expect("a valid puzzle");
        assert_eq!(puzzle.last_move.as_deref(), Some("f2g3"));
        assert_eq!(puzzle.solution, ["e6e7", "b2b1", "b3c1", "b1c1", "h6c1"]);
        assert!(puzzle.fen.contains(" w "), "White, the player, is to move");
    }

    #[test]
    fn the_lichess_step_follows_the_ranges_middle() {
        assert_eq!(lichess_difficulty(600, 900), "easiest");
        assert_eq!(lichess_difficulty(1300, 1700), "normal");
        assert_eq!(lichess_difficulty(2000, 2400), "hardest");
    }
}
