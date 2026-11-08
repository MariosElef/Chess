use gpui_kit::{App, Global};

use crate::enums::settings::{BoardTheme, PieceSet};

/// The board's theme and pieces, chosen in Settings and read by everything that draws a board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BoardStyle {
    pub theme: BoardTheme,
    pub pieces: PieceSet,
}

impl Global for BoardStyle {}

impl BoardStyle {
    /// The chosen style; the defaults until the app has set one.
    pub fn current(cx: &App) -> Self {
        cx.try_global::<Self>().copied().unwrap_or_default()
    }
}
