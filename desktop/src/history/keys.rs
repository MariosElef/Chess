use gpui_kit::{App, KeyBinding};

gpui_kit::actions!(
    history,
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

/// The key context every page with a game history declares, so the arrows move through that game.
pub const CONTEXT: &str = "GameHistory";

/// The arrow keys Lichess uses to move through a game.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", PreviousPosition, Some(CONTEXT)),
        KeyBinding::new("right", NextPosition, Some(CONTEXT)),
        KeyBinding::new("up", FirstPosition, Some(CONTEXT)),
        KeyBinding::new("down", LivePosition, Some(CONTEXT)),
    ]);
}
