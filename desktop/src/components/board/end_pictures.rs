/// The game-over badges' pictures, by asset path.
const PICTURES: [(&str, &[u8]); 9] = [
    (
        "end/winner.png",
        include_bytes!("../../../assets/end/winner.png"),
    ),
    (
        "end/checkmate_white.png",
        include_bytes!("../../../assets/end/checkmate_white.png"),
    ),
    (
        "end/checkmate_black.png",
        include_bytes!("../../../assets/end/checkmate_black.png"),
    ),
    (
        "end/resign_white.png",
        include_bytes!("../../../assets/end/resign_white.png"),
    ),
    (
        "end/resign_black.png",
        include_bytes!("../../../assets/end/resign_black.png"),
    ),
    (
        "end/timeout_white.png",
        include_bytes!("../../../assets/end/timeout_white.png"),
    ),
    (
        "end/timeout_black.png",
        include_bytes!("../../../assets/end/timeout_black.png"),
    ),
    (
        "end/draw_white.png",
        include_bytes!("../../../assets/end/draw_white.png"),
    ),
    (
        "end/draw_black.png",
        include_bytes!("../../../assets/end/draw_black.png"),
    ),
];

/// The picture at `path`, if it is one of the game-over badges.
pub fn end_picture(path: &str) -> Option<&'static [u8]> {
    PICTURES
        .iter()
        .find(|(pictured, _)| *pictured == path)
        .map(|(_, bytes)| *bytes)
}

/// Every game-over badge's path, for listing.
pub fn end_picture_paths() -> impl Iterator<Item = &'static str> {
    PICTURES.iter().map(|(path, _)| *path)
}
