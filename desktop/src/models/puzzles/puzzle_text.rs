use serde::Deserialize;

use super::puzzle::Puzzle;

/// A puzzle as the server sends it: the position and moves still as text.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PuzzleText {
    pub id: String,
    pub rating: u32,
    pub plays: u32,
    pub themes: Vec<String>,
    pub fen: String,
    pub last_move: Option<String>,
    pub solution: Vec<String>,
}

impl PuzzleText {
    /// The position and moves read; `Err` names what did not parse.
    pub fn read(&self) -> Result<Puzzle, String> {
        let unreadable = |what: &str| format!("puzzle {}: bad {what}", self.id);
        let position = self.fen.parse().map_err(|_| unreadable("FEN"))?;
        let last_move = self
            .last_move
            .as_deref()
            .map(str::parse)
            .transpose()
            .map_err(|_| unreadable("last move"))?;
        let solution = self
            .solution
            .iter()
            .map(|mv| mv.parse())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| unreadable("solution"))?;
        Ok(Puzzle {
            id: self.id.clone(),
            rating: self.rating,
            plays: self.plays,
            themes: self.themes.clone(),
            position,
            last_move,
            solution,
        })
    }
}
