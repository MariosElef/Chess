use std::{sync::Arc, time::Duration};

use reqwest::Client;
use serde::Deserialize;

use crate::{
    errors::{AppError, ChessCode},
    models::puzzles::puzzle::GetPuzzleModel,
};

/// Lichess's own answer takes no longer than this, or puzzles count as unavailable.
const TIMEOUT: Duration = Duration::from_secs(10);
/// Lichess asks API callers to say who they are.
const USER_AGENT: &str = "chess-backend (a personal chess app)";
const LICHESS: ChessCode = ChessCode::LichessUnavailable;

/// Lichess's public API: puzzles from its CC0 puzzle database.
#[derive(Clone)]
pub struct LichessClient {
    http: Client,
    base_url: Arc<str>,
}

impl LichessClient {
    pub fn new(base_url: &str) -> anyhow::Result<Self> {
        let http = Client::builder()
            .timeout(TIMEOUT)
            .user_agent(USER_AGENT)
            .build()?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').into(),
        })
    }

    /// A random Lichess puzzle at `difficulty`, easiest to hardest, as for a 1500 player.
    pub async fn next_puzzle(&self, difficulty: &str) -> Result<GetPuzzleModel, AppError> {
        let response = self
            .http
            .get(format!(
                "{}/api/puzzle/next?difficulty={difficulty}",
                self.base_url
            ))
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|error| AppError::unavailable(LICHESS, error))?
            .error_for_status()
            .map_err(|error| AppError::unavailable(LICHESS, error))?;
        let next: NextPuzzle = response
            .json()
            .await
            .map_err(|error| AppError::unavailable(LICHESS, error))?;
        Ok(next.puzzle.into_model())
    }
}

/// `GET /api/puzzle/next`, as Lichess sends it; only what the app uses is read.
#[derive(Deserialize)]
struct NextPuzzle {
    puzzle: LichessPuzzle,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LichessPuzzle {
    id: String,
    rating: i32,
    plays: i32,
    themes: Vec<String>,
    /// The position after the opponent's last move: the player is to move.
    fen: String,
    last_move: Option<String>,
    solution: Vec<String>,
}

impl LichessPuzzle {
    fn into_model(self) -> GetPuzzleModel {
        GetPuzzleModel {
            id: self.id,
            rating: self.rating,
            plays: self.plays,
            themes: self.themes,
            fen: self.fen,
            last_move: self.last_move,
            solution: self.solution,
        }
    }
}
