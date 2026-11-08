/// Puzzle Battle's shield: its asset path and bytes.
pub const BATTLE_PICTURE: &str = "puzzles/battle.svg";
const PICTURES: [(&str, &[u8]); 1] = [(
    BATTLE_PICTURE,
    include_bytes!("../../../assets/puzzles/battle.svg"),
)];

/// The picture at `path`, if it is one of the Puzzles page's own.
pub fn puzzle_picture(path: &str) -> Option<&'static [u8]> {
    PICTURES
        .iter()
        .find(|(pictured, _)| *pictured == path)
        .map(|(_, bytes)| *bytes)
}

/// Every one of them, for listing.
pub fn puzzle_picture_paths() -> impl Iterator<Item = &'static str> {
    PICTURES.iter().map(|(path, _)| *path)
}
