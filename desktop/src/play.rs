mod captured_row;
mod clock_face;
mod game_setup;
mod play_page;
mod status;
mod time_controls;

pub use play_page::PlayPage;

use chess_domain::Game;

/// Play asks for its game to be opened on the analysis board, at the position it was showing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzeGame {
    pub game: Game,
    pub viewing: usize,
}
