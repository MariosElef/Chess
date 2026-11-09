//! The Settings page's own pictures: the sections' and the appearance choices'.

/// The Appearance section's palette.
pub const APPEARANCE_PICTURE: &str = "settings/appearance.svg";
/// The Board section's knight: the staunty set's white one, one of the board's own pieces.
pub const BOARD_PICTURE: &str = "pieces/staunty/wN.svg";
/// The Sounds section's speaker.
pub const SOUNDS_PICTURE: &str = "settings/sounds.svg";
/// Stockfish's fish: over its Settings section and its panel in Analysis.
pub const STOCKFISH_PICTURE: &str = "settings/stockfish.svg";
/// The appearance choices: following the system, light and dark.
pub const SYSTEM_PICTURE: &str = "settings/system.svg";
pub const LIGHT_PICTURE: &str = "settings/light.svg";
pub const DARK_PICTURE: &str = "settings/dark.svg";

/// Each picture of the page's own: its asset path and its bytes.
const PICTURES: [(&str, &[u8]); 6] = [
    (
        APPEARANCE_PICTURE,
        include_bytes!("../../../assets/settings/appearance.svg"),
    ),
    (
        SOUNDS_PICTURE,
        include_bytes!("../../../assets/settings/sounds.svg"),
    ),
    (
        STOCKFISH_PICTURE,
        include_bytes!("../../../assets/settings/stockfish.svg"),
    ),
    (
        SYSTEM_PICTURE,
        include_bytes!("../../../assets/settings/system.svg"),
    ),
    (
        LIGHT_PICTURE,
        include_bytes!("../../../assets/settings/light.svg"),
    ),
    (
        DARK_PICTURE,
        include_bytes!("../../../assets/settings/dark.svg"),
    ),
];

/// The picture at `path`, if it is one of the Settings page's.
pub fn settings_picture(path: &str) -> Option<&'static [u8]> {
    PICTURES
        .iter()
        .find(|(pictured, _)| *pictured == path)
        .map(|(_, bytes)| *bytes)
}

/// Every Settings picture's path, for listing.
pub fn settings_picture_paths() -> impl Iterator<Item = &'static str> {
    PICTURES.iter().map(|(path, _)| *path)
}
