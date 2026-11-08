use serde::Serialize;

use crate::models::puzzles::summary::BestRushModel;

/// Everything the player has done: each game mode and puzzles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GetStatsModel {
    /// Bullet, blitz, rapid and classical, in that order.
    pub modes: Vec<ModeStatsModel>,
    pub puzzles: PuzzleStatsModel,
}

/// One game mode: the rating, rounded, its best, and how the finished games went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModeStatsModel {
    pub category: String,
    /// `None` before the first rated game.
    pub rating: Option<i32>,
    pub provisional: bool,
    pub best: Option<i32>,
    pub games: i64,
    pub wins: i64,
    pub losses: i64,
    pub draws: i64,
    /// The latest ratings, rounded and oldest first, for its graph.
    pub history: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PuzzleStatsModel {
    pub rating: Option<i32>,
    pub provisional: bool,
    pub best: Option<i32>,
    pub attempted: i64,
    pub solved: i64,
    pub best_rush: BestRushModel,
    /// The Puzzle Battle rating, before the first rated battle `None`.
    pub battle_rating: Option<i32>,
    pub battle_provisional: bool,
    /// The latest puzzle ratings, rounded and oldest first, for its graph.
    pub history: Vec<i32>,
}

/// A rating category with the player's rating in it, if they have one.
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryRatingRow {
    pub category: String,
    pub rating: Option<f64>,
    pub deviation: Option<f64>,
    pub volatility: Option<f64>,
    pub best: Option<f64>,
}

/// One rating the player had in a category.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryRow {
    pub category: String,
    pub rating: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameResultsRow {
    pub category: String,
    pub games: i64,
    pub wins: i64,
    pub draws: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PuzzleCountsRow {
    pub attempted: i64,
    pub solved: i64,
}
