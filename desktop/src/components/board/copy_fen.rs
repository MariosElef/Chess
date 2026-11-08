use chess_domain::Position;
use gpui_kit::{component::clipboard::Clipboard, *};

/// A small button beside the board that copies the shown position's FEN.
pub fn copy_fen(position: &Position) -> Clipboard {
    Clipboard::new("copy-fen")
        .value(SharedString::from(position.to_string()))
        .tooltip("Copy FEN")
}
