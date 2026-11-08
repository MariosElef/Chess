use gpui_kit::{App, KeyBinding};

gpui_kit::actions!(
    play,
    [
        /// ←
        PreviousPosition,
        /// →
        NextPosition,
        /// ↑
        FirstPosition,
        /// ↓
        LivePosition,
    ]
);

/// The key context the Play page declares, so the arrows only move through a game while it is open.
pub const CONTEXT: &str = "PlayPage";

/// The arrow keys Lichess uses to move through a game.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", PreviousPosition, Some(CONTEXT)),
        KeyBinding::new("right", NextPosition, Some(CONTEXT)),
        KeyBinding::new("up", FirstPosition, Some(CONTEXT)),
        KeyBinding::new("down", LivePosition, Some(CONTEXT)),
    ]);
}
