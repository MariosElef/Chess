/// The bots' pictures, by asset path.
const PICTURES: [(&str, &[u8]); 5] = [
    (
        "bots/sporos.jpg",
        include_bytes!("../../../assets/bots/sporos.jpg"),
    ),
    (
        "bots/danis.jpg",
        include_bytes!("../../../assets/bots/danis.jpg"),
    ),
    (
        "bots/stefanos.jpg",
        include_bytes!("../../../assets/bots/stefanos.jpg"),
    ),
    (
        "bots/vasilis.jpg",
        include_bytes!("../../../assets/bots/vasilis.jpg"),
    ),
    (
        "bots/andreas.jpg",
        include_bytes!("../../../assets/bots/andreas.jpg"),
    ),
];

/// The picture at `path`, if it is one of the bots'.
pub fn bot_picture(path: &str) -> Option<&'static [u8]> {
    PICTURES
        .iter()
        .find(|(pictured, _)| *pictured == path)
        .map(|(_, bytes)| *bytes)
}

/// Every bot picture's path, for listing.
pub fn bot_picture_paths() -> impl Iterator<Item = &'static str> {
    PICTURES.iter().map(|(path, _)| *path)
}
