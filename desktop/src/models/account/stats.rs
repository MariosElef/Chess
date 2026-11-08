use serde::Deserialize;

use crate::models::puzzles::BestRush;

/// Everything the player has done: each game mode and puzzles.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Stats {
    /// Bullet, blitz, rapid and classical, in that order.
    pub modes: Vec<ModeStats>,
    pub puzzles: PuzzleStats,
}

/// One game mode: the rating, its best, and how the finished games went.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ModeStats {
    /// The server's key for the mode: `bullet`, `blitz`, `rapid` or `classical`.
    pub category: String,
    /// `None` before the first rated game.
    pub rating: Option<i32>,
    pub provisional: bool,
    pub best: Option<i32>,
    pub games: u32,
    pub wins: u32,
    pub losses: u32,
    pub draws: u32,
    /// The latest ratings, oldest first, for its graph.
    #[serde(default)]
    pub history: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PuzzleStats {
    pub rating: Option<i32>,
    pub provisional: bool,
    pub best: Option<i32>,
    pub attempted: u32,
    pub solved: u32,
    pub best_rush: BestRush,
    pub battle_rating: Option<i32>,
    pub battle_provisional: bool,
    /// The latest puzzle ratings, oldest first, for its graph.
    #[serde(default)]
    pub history: Vec<i32>,
}
