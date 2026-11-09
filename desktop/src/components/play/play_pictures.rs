/// The dice on the bot menu's Random colour.
pub const DICE_PICTURE: &str = "play/dice.svg";

/// Play's own pictures: each one's asset path and its bytes.
const PICTURES: [(&str, &[u8]); 1] = [(
    DICE_PICTURE,
    include_bytes!("../../../assets/play/dice.svg"),
)];

/// The picture at `path`, if it is one of Play's.
pub fn play_picture(path: &str) -> Option<&'static [u8]> {
    PICTURES
        .iter()
        .find(|(pictured, _)| *pictured == path)
        .map(|(_, bytes)| *bytes)
}

/// Every Play picture's path, for listing.
pub fn play_picture_paths() -> impl Iterator<Item = &'static str> {
    PICTURES.iter().map(|(path, _)| *path)
}
